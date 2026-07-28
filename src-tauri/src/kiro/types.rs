use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KiroProvider {
    BuilderId,
    Enterprise,
    Github,
    Google,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KiroAuthMethod {
    IdC,
    #[serde(rename = "social")]
    Social,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
    pub auth_method: KiroAuthMethod,
    pub provider: KiroProvider,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub region: String,
    pub start_url: Option<String>,
    pub profile_arn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroBonusUsage {
    pub code: String,
    pub name: String,
    pub current: f64,
    pub limit: f64,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroUsage {
    pub current: f64,
    pub limit: f64,
    pub base_current: f64,
    pub base_limit: f64,
    pub trial_current: f64,
    pub trial_limit: f64,
    pub trial_expires_at: Option<String>,
    pub bonuses: Vec<KiroBonusUsage>,
    pub subscription_title: String,
    pub subscription_type: String,
    pub next_reset_at: Option<String>,
    pub user_email: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroPoolAccount {
    pub id: i64,
    pub email: String,
    pub provider: KiroProvider,
    #[serde(alias = "auth_method")]
    pub auth_method: KiroAuthMethod,
    #[serde(alias = "access_token")]
    pub access_token: String,
    #[serde(alias = "refresh_token")]
    pub refresh_token: String,
    #[serde(alias = "expires_at")]
    pub expires_at: i64,
    #[serde(alias = "client_id")]
    pub client_id: Option<String>,
    #[serde(alias = "client_secret")]
    pub client_secret: Option<String>,
    pub region: String,
    #[serde(alias = "start_url")]
    pub start_url: Option<String>,
    #[serde(alias = "profile_arn")]
    pub profile_arn: Option<String>,
    #[serde(alias = "machine_id")]
    pub machine_id: String,
    #[serde(alias = "credit_quota")]
    pub credit_quota: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiroReservation {
    pub reservation_id: String,
    pub expires_at: i64,
    pub account: KiroPoolAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroSwitchOptions {
    pub force_close: bool,
    pub launch_after_switch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroSwitchResult {
    pub email: String,
    pub provider: KiroProvider,
    pub machine_id: String,
    pub deducted_credits: i32,
    pub account: KiroPoolAccount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_error: Option<String>,
}
