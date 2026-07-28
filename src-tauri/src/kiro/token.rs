use crate::kiro::types::{KiroAuthMethod, KiroCredentials, KiroPoolAccount};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

pub const KIRO_SOCIAL_REFRESH_URL: &str =
    "https://prod.us-east-1.auth.desktop.kiro.dev/refreshToken";

#[derive(Debug)]
pub struct RefreshRequest {
    pub url: String,
    pub body: Value,
}

#[derive(Debug, Clone)]
pub struct RefreshedCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

pub struct HttpTokenService {
    client: Client,
}

impl HttpTokenService {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl crate::kiro::switch::TokenService for HttpTokenService {
    async fn refresh(&self, account: &KiroPoolAccount) -> Result<RefreshedCredentials, String> {
        refresh_credentials(
            &self.client,
            &KiroCredentials {
                access_token: account.access_token.clone(),
                refresh_token: account.refresh_token.clone(),
                expires_at: String::new(),
                auth_method: account.auth_method.clone(),
                provider: account.provider.clone(),
                client_id: account.client_id.clone(),
                client_secret: account.client_secret.clone(),
                region: account.region.clone(),
                start_url: account.start_url.clone(),
                profile_arn: account.profile_arn.clone(),
            },
        )
        .await
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default = "default_expires_in")]
    expires_in: i64,
}

pub fn build_refresh_request(credentials: &KiroCredentials) -> Result<RefreshRequest, String> {
    if credentials.refresh_token.trim().is_empty() {
        return Err("Kiro refresh token is required".to_string());
    }
    match credentials.auth_method {
        KiroAuthMethod::Social => Ok(RefreshRequest {
            url: KIRO_SOCIAL_REFRESH_URL.to_string(),
            body: json!({ "refreshToken": credentials.refresh_token }),
        }),
        KiroAuthMethod::IdC => {
            let client_id = credentials
                .client_id
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Kiro IdC credentials require clientId".to_string())?;
            let client_secret = credentials
                .client_secret
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Kiro IdC credentials require clientSecret".to_string())?;
            Ok(RefreshRequest {
                url: format!("https://oidc.{}.amazonaws.com/token", credentials.region),
                body: json!({
                    "clientId": client_id,
                    "clientSecret": client_secret,
                    "refreshToken": credentials.refresh_token,
                    "grantType": "refresh_token"
                }),
            })
        }
    }
}

pub fn parse_refresh_response(
    bytes: &[u8],
    previous_refresh_token: &str,
) -> Result<RefreshedCredentials, String> {
    let response: RefreshResponse = serde_json::from_slice(bytes)
        .map_err(|error| format!("decode Kiro token refresh: {error}"))?;
    if response.access_token.is_empty() {
        return Err("Kiro token refresh returned no access token".to_string());
    }
    Ok(RefreshedCredentials {
        access_token: response.access_token,
        refresh_token: if response.refresh_token.is_empty() {
            previous_refresh_token.to_string()
        } else {
            response.refresh_token
        },
        expires_in: if response.expires_in > 0 {
            response.expires_in
        } else {
            default_expires_in()
        },
    })
}

pub async fn refresh_credentials(
    client: &Client,
    credentials: &KiroCredentials,
) -> Result<RefreshedCredentials, String> {
    let refresh = build_refresh_request(credentials)?;
    let response = client
        .post(&refresh.url)
        .header("User-Agent", "KiroIDE-0.6.18")
        .json(&refresh.body)
        .send()
        .await
        .map_err(|error| format!("Kiro token refresh unavailable: {error}"))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("read Kiro token refresh: {error}"))?;
    if !status.is_success() {
        return Err(format!(
            "Kiro token refresh rejected credentials (HTTP {}): {}",
            status.as_u16(),
            String::from_utf8_lossy(&bytes[..bytes.len().min(4096)])
        ));
    }
    parse_refresh_response(&bytes, &credentials.refresh_token)
}

fn default_expires_in() -> i64 {
    3600
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kiro::types::{KiroAuthMethod, KiroCredentials, KiroProvider};

    fn credentials(auth_method: KiroAuthMethod) -> KiroCredentials {
        KiroCredentials {
            access_token: String::new(),
            refresh_token: "old-refresh".to_string(),
            expires_at: String::new(),
            auth_method,
            provider: KiroProvider::BuilderId,
            client_id: Some("client-id".to_string()),
            client_secret: Some("client-secret".to_string()),
            region: "eu-west-1".to_string(),
            start_url: None,
            profile_arn: None,
        }
    }

    #[test]
    fn oidc_refresh_request_uses_aws_payload() {
        let request = build_refresh_request(&credentials(KiroAuthMethod::IdC)).unwrap();
        assert_eq!(request.url, "https://oidc.eu-west-1.amazonaws.com/token");
        assert_eq!(request.body["grantType"], "refresh_token");
        assert_eq!(request.body["clientId"], "client-id");
        assert_eq!(request.body["clientSecret"], "client-secret");
    }

    #[test]
    fn social_refresh_request_uses_kiro_auth_service() {
        let request = build_refresh_request(&credentials(KiroAuthMethod::Social)).unwrap();
        assert_eq!(request.url, KIRO_SOCIAL_REFRESH_URL);
        assert_eq!(request.body["refreshToken"], "old-refresh");
        assert!(request.body.get("clientId").is_none());
    }

    #[test]
    fn refresh_response_uses_rotated_refresh_token() {
        let refreshed = parse_refresh_response(
            br#"{"accessToken":"new-access","refreshToken":"rotated","expiresIn":3600}"#,
            "old-refresh",
        )
        .unwrap();
        assert_eq!(refreshed.access_token, "new-access");
        assert_eq!(refreshed.refresh_token, "rotated");
        assert_eq!(refreshed.expires_in, 3600);
    }
}
