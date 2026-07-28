use crate::api::ApiClient;
use crate::config;
use crate::database::Database;
use crate::kiro::credentials::FilesystemCredentialStore;
use crate::kiro::machine_id::WindowsMachineIdStore;
use crate::kiro::process::{discover_kiro_executable, SystemKiroProcess};
use crate::kiro::switch::{KiroProcess, MachineIdStore, ReservationService, SwitchCoordinator};
use crate::kiro::token::HttpTokenService;
use crate::kiro::types::{
    KiroPoolAccount, KiroReservation, KiroSwitchOptions, KiroSwitchResult, KiroUsage,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use tauri::State;

const KIRO_PATH_KEY: &str = "system.kiro.executable_path";
const ORIGINAL_MACHINE_ID_KEY: &str = "system.kiro.original_machine_id";

#[derive(Deserialize)]
struct BackendResponse<T> {
    status: i32,
    msg: String,
    data: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiroStatus {
    pub running: bool,
    pub executable_path: String,
    pub authenticated: bool,
    pub provider: Option<String>,
    pub auth_method: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalTokenDocument {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    profile_arn: Option<String>,
    region: Option<String>,
    provider: Option<String>,
    auth_method: Option<String>,
    expires_at: Option<String>,
}

impl LocalTokenDocument {
    fn is_authenticated(&self) -> bool {
        !self.access_token.trim().is_empty()
            && self
                .refresh_token
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }
}

struct HttpReservationService<'a> {
    client: &'a ApiClient,
}

#[async_trait]
impl ReservationService for HttpReservationService<'_> {
    async fn reserve(&self) -> Result<KiroReservation, String> {
        let response = self
            .client
            .post(format!(
                "{}/kiro/accounts/reservations",
                config::get_auth_api_url()
            ))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|error| format!("reserve Kiro account: {error}"))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("read Kiro reservation response: {error}"))?;
        decode_api_data(&bytes)
    }

    async fn commit(
        &self,
        id: &str,
        credentials: &crate::kiro::types::KiroCredentials,
    ) -> Result<(), String> {
        let response = self
            .client
            .post(format!(
                "{}/kiro/accounts/reservations/{id}/commit",
                config::get_auth_api_url()
            ))
            .json(&serde_json::json!({
                "access_token": credentials.access_token,
                "refresh_token": credentials.refresh_token,
                "expires_at": chrono::DateTime::parse_from_rfc3339(&credentials.expires_at)
                    .map_err(|error| format!("parse refreshed Kiro expiry: {error}"))?
                    .timestamp()
            }))
            .send()
            .await
            .map_err(|error| format!("commit Kiro reservation: {error}"))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("read Kiro commit response: {error}"))?;
        decode_api_success(&bytes)
    }

    async fn release(&self, id: &str) -> Result<(), String> {
        let response = self
            .client
            .delete(format!(
                "{}/kiro/accounts/reservations/{id}",
                config::get_auth_api_url()
            ))
            .send()
            .await
            .map_err(|error| format!("release Kiro reservation: {error}"))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("read Kiro release response: {error}"))?;
        decode_api_success(&bytes)
    }
}

fn decode_api_data<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let response: BackendResponse<T> = serde_json::from_slice(bytes)
        .map_err(|error| format!("decode backend response: {error}"))?;
    if response.status != 200 {
        return Err(response.msg);
    }
    response
        .data
        .ok_or_else(|| "backend response contains no data".to_string())
}

fn decode_api_success(bytes: &[u8]) -> Result<(), String> {
    let response: BackendResponse<serde_json::Value> = serde_json::from_slice(bytes)
        .map_err(|error| format!("decode backend response: {error}"))?;
    if response.status == 200 {
        Ok(())
    } else {
        Err(response.msg)
    }
}

fn credential_cache_directory_from(home: &Path) -> PathBuf {
    home.join(".aws").join("sso").join("cache")
}

fn credential_cache_directory() -> Result<PathBuf, String> {
    let home = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| "USER_HOME_NOT_FOUND".to_string())?;
    Ok(credential_cache_directory_from(&home))
}

fn read_local_token_document() -> Result<LocalTokenDocument, String> {
    let token_path = credential_cache_directory()?.join("kiro-auth-token.json");
    serde_json::from_slice(
        &std::fs::read(&token_path)
            .map_err(|error| format!("read {}: {error}", token_path.display()))?,
    )
    .map_err(|error| format!("decode Kiro credentials: {error}"))
}

fn configured_kiro_path(database: &Database) -> Result<PathBuf, String> {
    let configured = database
        .get_item(KIRO_PATH_KEY)
        .map_err(|error| format!("read Kiro path: {error}"))?
        .map(PathBuf::from);
    discover_kiro_executable(configured.as_deref())
}

fn remember_original_machine_id(
    database: &Database,
    machine_ids: &WindowsMachineIdStore,
) -> Result<(), String> {
    if database
        .get_item(ORIGINAL_MACHINE_ID_KEY)
        .map_err(|error| format!("read original MachineGuid: {error}"))?
        .is_none()
    {
        let current = machine_ids.current()?;
        database
            .set_item(ORIGINAL_MACHINE_ID_KEY, &current)
            .map_err(|error| format!("save original MachineGuid: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn switch_kiro_account(
    client: State<'_, ApiClient>,
    database: State<'_, Database>,
    options: KiroSwitchOptions,
) -> Result<KiroSwitchResult, String> {
    let machine_ids = WindowsMachineIdStore;
    remember_original_machine_id(&database, &machine_ids)?;
    let executable = configured_kiro_path(&database)?;
    let coordinator = SwitchCoordinator::new(
        HttpReservationService { client: &client },
        HttpTokenService::new(Client::new()),
        machine_ids,
        FilesystemCredentialStore::new(credential_cache_directory()?),
        SystemKiroProcess::new(executable),
    );
    coordinator.switch(options).await
}

#[tauri::command]
pub async fn list_owned_kiro_accounts(
    client: State<'_, ApiClient>,
) -> Result<Vec<KiroPoolAccount>, String> {
    let response = client
        .get(format!("{}/kiro/accounts", config::get_auth_api_url()))
        .send()
        .await
        .map_err(|error| format!("list cloud Kiro accounts: {error}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("read cloud Kiro accounts response: {error}"))?;
    decode_api_data(&bytes)
}

#[tauri::command]
pub async fn delete_owned_kiro_account(
    client: State<'_, ApiClient>,
    account_id: i64,
) -> Result<(), String> {
    let response = client
        .delete(format!(
            "{}/kiro/accounts/{account_id}",
            config::get_auth_api_url()
        ))
        .send()
        .await
        .map_err(|error| format!("delete cloud Kiro account: {error}"))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("read delete Kiro account response: {error}"))?;
    decode_api_success(&bytes)
}

#[tauri::command]
pub async fn switch_owned_kiro_account(
    client: State<'_, ApiClient>,
    database: State<'_, Database>,
    account: KiroPoolAccount,
    options: KiroSwitchOptions,
) -> Result<KiroSwitchResult, String> {
    let machine_ids = WindowsMachineIdStore;
    remember_original_machine_id(&database, &machine_ids)?;
    let executable = configured_kiro_path(&database)?;
    let coordinator = SwitchCoordinator::new(
        NoReservationService,
        HttpTokenService::new(Client::new()),
        machine_ids,
        FilesystemCredentialStore::new(credential_cache_directory()?),
        SystemKiroProcess::new(executable),
    );
    let mut result = coordinator.switch_owned(account, options).await?;
    let response = client
        .patch(format!(
            "{}/kiro/accounts/{}/credentials",
            config::get_auth_api_url(),
            result.account.id
        ))
        .json(&serde_json::json!({
            "access_token": result.account.access_token,
            "refresh_token": result.account.refresh_token,
            "expires_at": result.account.expires_at
        }))
        .send()
        .await;
    result.sync_error = match response {
        Ok(response) => match response.bytes().await {
            Ok(bytes) => decode_api_success(&bytes).err(),
            Err(error) => Some(format!("read cloud credential update response: {error}")),
        },
        Err(error) => Some(format!("update cloud Kiro credentials: {error}")),
    };
    Ok(result)
}

struct NoReservationService;

#[async_trait]
impl ReservationService for NoReservationService {
    async fn reserve(&self) -> Result<KiroReservation, String> {
        Err("reservation is unavailable for local account switching".to_string())
    }
    async fn commit(
        &self,
        _id: &str,
        _credentials: &crate::kiro::types::KiroCredentials,
    ) -> Result<(), String> {
        Err("reservation is unavailable for local account switching".to_string())
    }
    async fn release(&self, _id: &str) -> Result<(), String> {
        Err("reservation is unavailable for local account switching".to_string())
    }
}

#[tauri::command]
pub fn get_kiro_status(database: State<'_, Database>) -> Result<KiroStatus, String> {
    let executable = configured_kiro_path(&database)?;
    let process = SystemKiroProcess::new(executable.clone());
    let local_token = read_local_token_document().ok();
    Ok(KiroStatus {
        running: process.is_running(),
        executable_path: executable.to_string_lossy().into_owned(),
        authenticated: local_token
            .as_ref()
            .is_some_and(LocalTokenDocument::is_authenticated),
        provider: local_token
            .as_ref()
            .and_then(|token| token.provider.clone()),
        auth_method: local_token
            .as_ref()
            .and_then(|token| token.auth_method.clone()),
        expires_at: local_token.and_then(|token| token.expires_at),
    })
}

#[tauri::command]
pub async fn get_kiro_usage() -> Result<KiroUsage, String> {
    let token = read_local_token_document()?;
    let machine_id = WindowsMachineIdStore.current().ok();
    crate::kiro::usage::get_usage(
        &Client::new(),
        &token.access_token,
        token.region.as_deref().unwrap_or("us-east-1"),
        token.profile_arn.as_deref(),
        machine_id.as_deref(),
    )
    .await
}

#[tauri::command]
pub fn close_kiro(database: State<'_, Database>) -> Result<(), String> {
    SystemKiroProcess::new(configured_kiro_path(&database)?).close()
}

#[tauri::command]
pub fn launch_kiro(database: State<'_, Database>) -> Result<(), String> {
    SystemKiroProcess::new(configured_kiro_path(&database)?).launch()
}

#[tauri::command]
pub fn set_kiro_path(database: State<'_, Database>, path: String) -> Result<(), String> {
    let executable = PathBuf::from(path.trim());
    if !executable.is_file()
        || !executable
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("kiro.exe"))
    {
        return Err("INVALID_KIRO_EXECUTABLE".to_string());
    }
    database
        .set_item(KIRO_PATH_KEY, &executable.to_string_lossy())
        .map_err(|error| format!("save Kiro path: {error}"))
}

#[tauri::command]
pub fn get_machine_id() -> Result<String, String> {
    WindowsMachineIdStore.current()
}

#[tauri::command]
pub fn restore_original_machine_id(database: State<'_, Database>) -> Result<String, String> {
    let original = database
        .get_item(ORIGINAL_MACHINE_ID_KEY)
        .map_err(|error| format!("read original MachineGuid: {error}"))?
        .ok_or_else(|| "ORIGINAL_MACHINE_ID_NOT_FOUND".to_string())?;
    WindowsMachineIdStore.set(&original)?;
    Ok(original)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_reservation_response_from_go_service() {
        let body = br#"{
            "status": 200,
            "msg": "ok",
            "data": {
                "reservation_id": "reservation-1",
                "expires_at": 4102444800,
                "account": {
                    "id": 7,
                    "email": "builder@example.com",
                    "provider": "BuilderId",
                    "auth_method": "IdC",
                    "access_token": "access",
                    "refresh_token": "refresh",
                    "expires_at": 4102444800,
                    "client_id": "client",
                    "client_secret": "secret",
                    "region": "us-east-1",
                    "machine_id": "11111111-1111-4111-8111-111111111111",
                    "credit_quota": 50
                }
            }
        }"#;

        let reservation = decode_api_data::<crate::kiro::types::KiroReservation>(body).unwrap();
        assert_eq!(reservation.reservation_id, "reservation-1");
        assert_eq!(reservation.account.email, "builder@example.com");
    }

    #[test]
    fn preserves_backend_error_message() {
        let error = decode_api_data::<serde_json::Value>(
            br#"{"status":409,"msg":"Kiro account reservation is unavailable"}"#,
        )
        .unwrap_err();

        assert_eq!(error, "Kiro account reservation is unavailable");
    }

    #[test]
    fn cache_directory_uses_aws_sso_cache() {
        let home = std::path::Path::new("C:/Users/tester");
        assert_eq!(
            credential_cache_directory_from(home),
            home.join(".aws").join("sso").join("cache")
        );
    }

    #[test]
    fn reads_official_camel_case_local_token_document() {
        let token: LocalTokenDocument = serde_json::from_value(serde_json::json!({
            "accessToken": "local-access",
            "refreshToken": "local-refresh",
            "expiresAt": "2026-07-28T18:59:02.391Z",
            "clientIdHash": "registration-hash",
            "authMethod": "IdC",
            "provider": "BuilderId",
            "region": "us-east-1",
            "profileArn": "arn:aws:codewhisperer:us-east-1:123:profile/example"
        }))
        .unwrap();

        assert_eq!(token.access_token, "local-access");
        assert_eq!(token.region.as_deref(), Some("us-east-1"));
        assert!(token
            .profile_arn
            .as_deref()
            .unwrap()
            .contains("profile/example"));
        assert_eq!(token.provider.as_deref(), Some("BuilderId"));
        assert_eq!(token.auth_method.as_deref(), Some("IdC"));
        assert_eq!(
            token.expires_at.as_deref(),
            Some("2026-07-28T18:59:02.391Z")
        );
        assert!(token.is_authenticated());
    }
}
