use crate::kiro::types::{KiroBonusUsage, KiroUsage};
use reqwest::{Client, StatusCode};
use serde::{de::Error as _, Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageLimitsResponse {
    #[serde(default)]
    usage_breakdown_list: Vec<UsageBreakdown>,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    next_date_reset: Option<String>,
    #[serde(default)]
    subscription_info: SubscriptionInfo,
    #[serde(default)]
    user_info: UserInfo,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionInfo {
    #[serde(default)]
    subscription_title: String,
    #[serde(default)]
    subscription_type: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfo {
    email: Option<String>,
    user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageBreakdown {
    resource_type: Option<String>,
    display_name: Option<String>,
    current_usage: Option<f64>,
    current_usage_with_precision: Option<f64>,
    usage_limit: Option<f64>,
    usage_limit_with_precision: Option<f64>,
    free_trial_info: Option<FreeTrialInfo>,
    #[serde(default)]
    bonuses: Vec<BonusUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FreeTrialInfo {
    free_trial_status: Option<String>,
    current_usage: Option<f64>,
    current_usage_with_precision: Option<f64>,
    usage_limit: Option<f64>,
    usage_limit_with_precision: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    free_trial_expiry: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BonusUsage {
    #[serde(default)]
    bonus_code: String,
    #[serde(default)]
    display_name: String,
    status: Option<String>,
    current_usage: Option<f64>,
    current_usage_with_precision: Option<f64>,
    usage_limit: Option<f64>,
    usage_limit_with_precision: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    expires_at: Option<String>,
}

fn deserialize_optional_timestamp<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value)),
        Some(serde_json::Value::Number(value)) => {
            let seconds = value
                .as_f64()
                .ok_or_else(|| D::Error::custom("Kiro timestamp is not a finite number"))?;
            if !seconds.is_finite() {
                return Err(D::Error::custom("Kiro timestamp is not finite"));
            }
            let whole_seconds = seconds.trunc() as i64;
            let nanos = ((seconds.fract().abs()) * 1_000_000_000.0).round() as u32;
            chrono::DateTime::<chrono::Utc>::from_timestamp(whole_seconds, nanos)
                .map(|value| Some(value.to_rfc3339()))
                .ok_or_else(|| D::Error::custom("Kiro timestamp is outside the supported range"))
        }
        Some(_) => Err(D::Error::custom(
            "Kiro timestamp must be a string or Unix timestamp",
        )),
    }
}

pub fn parse_usage(bytes: &[u8]) -> Result<KiroUsage, String> {
    let response: UsageLimitsResponse = serde_json::from_slice(bytes)
        .map_err(|error| format!("decode Kiro usage response: {error}"))?;
    let credit = response
        .usage_breakdown_list
        .iter()
        .find(|item| {
            item.resource_type.as_deref() == Some("CREDIT")
                || item.display_name.as_deref() == Some("Credits")
        })
        .ok_or_else(|| "Kiro usage response contains no credit resource".to_string())?;
    let base_current = precise(credit.current_usage_with_precision, credit.current_usage);
    let base_limit = precise(credit.usage_limit_with_precision, credit.usage_limit);
    let active_trial = credit
        .free_trial_info
        .as_ref()
        .filter(|trial| trial.free_trial_status.as_deref() == Some("ACTIVE"));
    let trial_current = active_trial
        .map(|trial| precise(trial.current_usage_with_precision, trial.current_usage))
        .unwrap_or_default();
    let trial_limit = active_trial
        .map(|trial| precise(trial.usage_limit_with_precision, trial.usage_limit))
        .unwrap_or_default();
    let bonuses: Vec<KiroBonusUsage> = credit
        .bonuses
        .iter()
        .filter(|bonus| bonus.status.as_deref() == Some("ACTIVE"))
        .map(|bonus| KiroBonusUsage {
            code: bonus.bonus_code.clone(),
            name: bonus.display_name.clone(),
            current: precise(bonus.current_usage_with_precision, bonus.current_usage),
            limit: precise(bonus.usage_limit_with_precision, bonus.usage_limit),
            expires_at: bonus.expires_at.clone(),
        })
        .collect();
    let bonus_current = bonuses.iter().map(|bonus| bonus.current).sum::<f64>();
    let bonus_limit = bonuses.iter().map(|bonus| bonus.limit).sum::<f64>();
    Ok(KiroUsage {
        current: base_current + trial_current + bonus_current,
        limit: base_limit + trial_limit + bonus_limit,
        base_current,
        base_limit,
        trial_current,
        trial_limit,
        trial_expires_at: active_trial.and_then(|trial| trial.free_trial_expiry.clone()),
        bonuses,
        subscription_title: response.subscription_info.subscription_title,
        subscription_type: response.subscription_info.subscription_type,
        next_reset_at: response.next_date_reset,
        user_email: response.user_info.email,
        user_id: response.user_info.user_id,
    })
}

pub fn usage_base_urls(region: &str) -> [&'static str; 2] {
    if region.starts_with("eu-") {
        [
            "https://q.eu-central-1.amazonaws.com",
            "https://q.us-east-1.amazonaws.com",
        ]
    } else {
        [
            "https://q.us-east-1.amazonaws.com",
            "https://q.eu-central-1.amazonaws.com",
        ]
    }
}

pub async fn get_usage(
    client: &Client,
    access_token: &str,
    region: &str,
    profile_arn: Option<&str>,
    machine_id: Option<&str>,
) -> Result<KiroUsage, String> {
    for (index, base) in usage_base_urls(region).iter().enumerate() {
        let mut request = client
            .get(format!("{base}/getUsageLimits"))
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .header("User-Agent", kiro_user_agent(machine_id))
            .header("x-amz-user-agent", kiro_amz_user_agent(machine_id))
            .query(&[
                ("origin", "AI_EDITOR"),
                ("resourceType", "AGENTIC_REQUEST"),
                ("isEmailRequired", "true"),
            ]);
        if let Some(arn) = profile_arn.filter(|value| !value.is_empty()) {
            request = request.query(&[("profileArn", arn)]);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("Kiro usage service unavailable: {error}"))?;
        let status = response.status();
        if status == StatusCode::FORBIDDEN && index == 0 {
            continue;
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("read Kiro usage response: {error}"))?;
        if !status.is_success() {
            return Err(format!(
                "Kiro usage request failed (HTTP {}): {}",
                status.as_u16(),
                String::from_utf8_lossy(&bytes[..bytes.len().min(4096)])
            ));
        }
        return parse_usage(&bytes);
    }
    Err("Kiro usage request failed on all regional endpoints".to_string())
}

fn precise(preferred: Option<f64>, fallback: Option<f64>) -> f64 {
    preferred.or(fallback).unwrap_or_default()
}

fn kiro_user_agent(machine_id: Option<&str>) -> String {
    let suffix = machine_id
        .map(|id| format!("KiroIDE-0.6.18-{id}"))
        .unwrap_or_else(|| "KiroIDE-0.6.18".to_string());
    format!(
        "aws-sdk-js/1.0.18 ua/2.1 os/windows lang/js api/codewhispererstreaming#1.0.18 {suffix}"
    )
}

fn kiro_amz_user_agent(machine_id: Option<&str>) -> String {
    machine_id
        .map(|id| format!("aws-sdk-js/1.0.18 KiroIDE 0.6.18 {id}"))
        .unwrap_or_else(|| "aws-sdk-js/1.0.18 KiroIDE-0.6.18".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_include_active_trial_and_bonuses() {
        let usage = parse_usage(include_str!("fixtures/usage_limits.json").as_bytes()).unwrap();
        assert_eq!(usage.limit, 150.0);
        assert_eq!(usage.current, 27.5);
        assert_eq!(usage.base_limit, 100.0);
        assert_eq!(usage.trial_limit, 25.0);
        assert_eq!(usage.bonuses.len(), 1);
        assert_eq!(usage.subscription_title, "KIRO PRO+");
        assert_eq!(usage.user_email.as_deref(), Some("healthy@example.com"));
    }

    #[test]
    fn eu_regions_use_eu_endpoint_first() {
        assert_eq!(
            usage_base_urls("eu-west-1"),
            [
                "https://q.eu-central-1.amazonaws.com",
                "https://q.us-east-1.amazonaws.com"
            ]
        );
    }

    #[test]
    fn parses_numeric_reset_timestamp_from_live_kiro_response() {
        let usage = parse_usage(
            br#"{
                "nextDateReset": 1785542400.0,
                "subscriptionInfo": {"subscriptionTitle": "KIRO FREE"},
                "userInfo": {"email": "local@example.com"},
                "usageBreakdownList": [{
                    "resourceType": "CREDIT",
                    "displayName": "Credit",
                    "currentUsage": 1,
                    "usageLimit": 50,
                    "bonuses": []
                }]
            }"#,
        )
        .unwrap();

        let reset =
            chrono::DateTime::parse_from_rfc3339(usage.next_reset_at.as_deref().unwrap()).unwrap();
        assert_eq!(reset.timestamp(), 1_785_542_400);
        assert_eq!(usage.user_email.as_deref(), Some("local@example.com"));
    }
}
