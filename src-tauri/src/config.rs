use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::RwLock;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub default_api_url: String,
    pub config_file_url: String,
    pub public_endpoints: Vec<String>,
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbKeyConfig {
    pub inbound_config_key: String,
    pub current_inbound_key: String,
    pub token_key: String,
    pub lang_key: String,
    pub dashboard_refresh_interval_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub ping_timeout_ms: u64,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub dashboard_refresh_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub db_keys: DbKeyConfig,
    pub timeouts: TimeoutConfig,
    pub scheduler: SchedulerConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api: ApiConfig {
                default_api_url: "http://127.0.0.1:8080/api".to_string(),
                config_file_url: "http://127.0.0.1:8080/api/inbound/config".to_string(),
                public_endpoints: vec![
                    "/login".to_string(),
                    "/register".to_string(),
                    "/emailRegister".to_string(),
                    "/checkUser".to_string(),
                    "/register/sendEmailCode".to_string(),
                    "/emailResetPassword".to_string(),
                    "/version".to_string(),
                    "/public/info".to_string(),
                ],
                verify_ssl: false,
            },
            db_keys: DbKeyConfig {
                inbound_config_key: "system.inbound.config".to_string(),
                current_inbound_key: "system.inbound.current".to_string(),
                token_key: "user.info.token".to_string(),
                lang_key: "user.info.lang".to_string(),
                dashboard_refresh_interval_key: "system.scheduler.dashboard_refresh_interval"
                    .to_string(),
            },
            timeouts: TimeoutConfig {
                ping_timeout_ms: 5000,
                request_timeout_secs: 10,
            },
            scheduler: SchedulerConfig {
                dashboard_refresh_interval: 300,
            },
        }
    }
}

lazy_static! {
    pub static ref CONFIG: RwLock<AppConfig> = RwLock::new(AppConfig::default());
}

pub fn init_config() -> Result<(), String> {
    Ok(())
}

pub fn get_default_api_url() -> String {
    CONFIG.read().unwrap().api.default_api_url.clone()
}

pub fn get_auth_api_url() -> String {
    env::var("KIROPOOL_AUTH_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080/api".to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn get_config_file_url() -> String {
    CONFIG.read().unwrap().api.config_file_url.clone()
}

pub fn get_request_timeout() -> Duration {
    Duration::from_secs(CONFIG.read().unwrap().timeouts.request_timeout_secs)
}

pub fn get_ping_timeout() -> Duration {
    Duration::from_millis(CONFIG.read().unwrap().timeouts.ping_timeout_ms)
}

pub fn is_public_endpoint(url: &str) -> bool {
    CONFIG
        .read()
        .unwrap()
        .api
        .public_endpoints
        .iter()
        .any(|endpoint| url.contains(endpoint))
}

pub fn get_scheduler_config() -> SchedulerConfig {
    CONFIG.read().unwrap().scheduler.clone()
}

pub fn get_db_key(key_name: &str) -> String {
    let config = CONFIG.read().unwrap();
    match key_name {
        "dashboard_refresh_interval" => config.db_keys.dashboard_refresh_interval_key.clone(),
        "inbound_config" => config.db_keys.inbound_config_key.clone(),
        "current_inbound" => config.db_keys.current_inbound_key.clone(),
        "token" => config.db_keys.token_key.clone(),
        "lang" => config.db_keys.lang_key.clone(),
        _ => panic!("Unknown key name: {key_name}"),
    }
}
