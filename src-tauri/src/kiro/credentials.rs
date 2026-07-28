use crate::kiro::types::{KiroAuthMethod, KiroCredentials, KiroProvider};
use chrono::{Duration, Utc};
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

pub const KIRO_SOCIAL_PROFILE_ARN: &str =
    "arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK";
pub const KIRO_BUILDER_PROFILE_ARN: &str =
    "arn:aws:codewhisperer:us-east-1:638616132270:profile/AAAACCCCXXXX";
const DEFAULT_START_URL: &str = "https://view.awsapps.com/start";
const KIRO_SCOPES: [&str; 5] = [
    "codewhisperer:completions",
    "codewhisperer:analysis",
    "codewhisperer:conversations",
    "codewhisperer:transformations",
    "codewhisperer:taskassist",
];

#[derive(Debug)]
struct FileBackup {
    path: PathBuf,
    contents: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct CredentialWrite {
    pub token_path: PathBuf,
    pub registration_path: Option<PathBuf>,
    token_backup: FileBackup,
    registration_backup: Option<FileBackup>,
}

pub struct FilesystemCredentialStore {
    cache_directory: PathBuf,
    latest_write: Mutex<Option<CredentialWrite>>,
}

impl FilesystemCredentialStore {
    pub fn new(cache_directory: PathBuf) -> Self {
        Self {
            cache_directory,
            latest_write: Mutex::new(None),
        }
    }
}

impl crate::kiro::switch::CredentialStore for FilesystemCredentialStore {
    fn write(&self, credentials: &KiroCredentials) -> Result<(), String> {
        let written = write_kiro_credentials(&self.cache_directory, credentials)?;
        *self
            .latest_write
            .lock()
            .map_err(|_| "Kiro credential rollback lock is poisoned".to_string())? = Some(written);
        Ok(())
    }

    fn restore(&self) -> Result<(), String> {
        let written = self
            .latest_write
            .lock()
            .map_err(|_| "Kiro credential rollback lock is poisoned".to_string())?
            .take();
        match written {
            Some(value) => restore_kiro_credentials(&value),
            None => Ok(()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenDocument<'a> {
    access_token: &'a str,
    refresh_token: &'a str,
    profile_arn: String,
    expires_at: &'a str,
    auth_method: &'a KiroAuthMethod,
    provider: &'a KiroProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistrationDocument<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    expires_at: String,
    scopes: &'static [&'static str],
}

pub fn client_id_hash(start_url: &str) -> String {
    let canonical = serde_json::json!({ "startUrl": start_url }).to_string();
    hex::encode(Sha1::digest(canonical.as_bytes()))
}

pub fn resolve_profile_arn(
    provider: &KiroProvider,
    region: &str,
    supplied: Option<&str>,
) -> String {
    if let Some(value) = supplied
        .filter(|value| !value.trim().is_empty() && value.trim() != KIRO_BUILDER_PROFILE_ARN)
    {
        return value.trim().to_string();
    }
    match provider {
        KiroProvider::Github | KiroProvider::Google => KIRO_SOCIAL_PROFILE_ARN.to_string(),
        KiroProvider::Enterprise => {
            let endpoint_region = if region.starts_with("eu-") {
                "eu-central-1"
            } else {
                "us-east-1"
            };
            format!("arn:aws:codewhisperer:{endpoint_region}:610548660232:profile/VNECVYCYYAWN")
        }
        KiroProvider::BuilderId => KIRO_BUILDER_PROFILE_ARN.to_string(),
    }
}

pub fn write_kiro_credentials(
    cache_directory: &Path,
    credentials: &KiroCredentials,
) -> Result<CredentialWrite, String> {
    fs::create_dir_all(cache_directory)
        .map_err(|error| format!("create Kiro credential directory: {error}"))?;
    let token_path = cache_directory.join("kiro-auth-token.json");
    let token_backup = backup_file(&token_path)?;
    let start_url = credentials
        .start_url
        .as_deref()
        .unwrap_or(DEFAULT_START_URL);
    let is_oidc = credentials.auth_method == KiroAuthMethod::IdC;
    let hash = is_oidc.then(|| client_id_hash(start_url));
    let token = TokenDocument {
        access_token: &credentials.access_token,
        refresh_token: &credentials.refresh_token,
        profile_arn: resolve_profile_arn(
            &credentials.provider,
            &credentials.region,
            credentials.profile_arn.as_deref(),
        ),
        expires_at: &credentials.expires_at,
        auth_method: &credentials.auth_method,
        provider: &credentials.provider,
        client_id_hash: hash.clone(),
        region: is_oidc.then_some(credentials.region.as_str()),
    };
    let token_bytes = serde_json::to_vec_pretty(&token)
        .map_err(|error| format!("serialize Kiro token: {error}"))?;
    atomic_write(&token_path, &token_bytes)?;

    let mut written = CredentialWrite {
        token_path,
        registration_path: None,
        token_backup,
        registration_backup: None,
    };
    if is_oidc {
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
        let registration_path = cache_directory.join(format!("{}.json", hash.unwrap()));
        let registration_backup = backup_file(&registration_path)?;
        let registration = RegistrationDocument {
            client_id,
            client_secret,
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
            scopes: &KIRO_SCOPES,
        };
        let registration_bytes = serde_json::to_vec_pretty(&registration)
            .map_err(|error| format!("serialize Kiro client registration: {error}"))?;
        if let Err(error) = atomic_write(&registration_path, &registration_bytes) {
            let _ = restore_file(&written.token_backup);
            return Err(error);
        }
        written.registration_path = Some(registration_path);
        written.registration_backup = Some(registration_backup);
    }
    Ok(written)
}

pub fn restore_kiro_credentials(written: &CredentialWrite) -> Result<(), String> {
    let mut errors = Vec::new();
    if let Some(backup) = &written.registration_backup {
        if let Err(error) = restore_file(backup) {
            errors.push(error);
        }
    }
    if let Err(error) = restore_file(&written.token_backup) {
        errors.push(error);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn backup_file(path: &Path) -> Result<FileBackup, String> {
    let contents = if path.exists() {
        Some(fs::read(path).map_err(|error| format!("back up {}: {error}", path.display()))?)
    } else {
        None
    };
    Ok(FileBackup {
        path: path.to_path_buf(),
        contents,
    })
}

fn restore_file(backup: &FileBackup) -> Result<(), String> {
    match &backup.contents {
        Some(contents) => atomic_write(&backup.path, contents),
        None => {
            if backup.path.exists() {
                fs::remove_file(&backup.path).map_err(|error| {
                    format!("remove {} during rollback: {error}", backup.path.display())
                })?;
            }
            Ok(())
        }
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::File::create(&temporary)
            .map_err(|error| format!("create {}: {error}", temporary.display()))?;
        file.write_all(contents)
            .map_err(|error| format!("write {}: {error}", temporary.display()))?;
        file.sync_all()
            .map_err(|error| format!("flush {}: {error}", temporary.display()))?;
        if path.exists() {
            fs::remove_file(path)
                .map_err(|error| format!("replace {}: {error}", path.display()))?;
        }
        fs::rename(&temporary, path).map_err(|error| format!("replace {}: {error}", path.display()))
    })();
    if result.is_err() && temporary.exists() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kiro::types::{KiroAuthMethod, KiroCredentials, KiroProvider};
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    fn credentials(provider: KiroProvider, auth_method: KiroAuthMethod) -> KiroCredentials {
        KiroCredentials {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: "2026-07-28T01:00:00Z".to_string(),
            auth_method,
            provider,
            client_id: Some("client-id".to_string()),
            client_secret: Some("client-secret".to_string()),
            region: "us-east-1".to_string(),
            start_url: Some("https://view.awsapps.com/start".to_string()),
            profile_arn: None,
        }
    }

    #[test]
    fn client_id_hash_matches_kiro_format() {
        assert_eq!(
            client_id_hash("https://view.awsapps.com/start"),
            "e909a0580879b06ece1202964fbe9dda95ea4ce3"
        );
    }

    #[test]
    fn social_credentials_omit_oidc_registration_fields() {
        let directory = tempdir().unwrap();
        let input = credentials(KiroProvider::Github, KiroAuthMethod::Social);
        let written = write_kiro_credentials(directory.path(), &input).unwrap();
        let token: Value = serde_json::from_slice(&fs::read(&written.token_path).unwrap()).unwrap();

        assert_eq!(token["authMethod"], "social");
        assert_eq!(token["provider"], "Github");
        assert_eq!(token["profileArn"], KIRO_SOCIAL_PROFILE_ARN);
        assert!(token.get("clientIdHash").is_none());
        assert!(written.registration_path.is_none());
    }

    #[test]
    fn oidc_credentials_write_client_registration_and_restore_backup() {
        let directory = tempdir().unwrap();
        let token_path = directory.path().join("kiro-auth-token.json");
        fs::write(&token_path, b"old-token").unwrap();
        let input = credentials(KiroProvider::BuilderId, KiroAuthMethod::IdC);

        let written = write_kiro_credentials(directory.path(), &input).unwrap();
        let token: Value = serde_json::from_slice(&fs::read(&written.token_path).unwrap()).unwrap();
        assert_eq!(
            token["clientIdHash"],
            "e909a0580879b06ece1202964fbe9dda95ea4ce3"
        );
        assert_eq!(token["profileArn"], KIRO_BUILDER_PROFILE_ARN);

        let registration_path = written.registration_path.as_ref().unwrap();
        let registration: Value =
            serde_json::from_slice(&fs::read(registration_path).unwrap()).unwrap();
        assert_eq!(registration["clientId"], "client-id");
        assert_eq!(registration["clientSecret"], "client-secret");
        assert_eq!(registration["scopes"].as_array().unwrap().len(), 5);

        restore_kiro_credentials(&written).unwrap();
        assert_eq!(fs::read(token_path).unwrap(), b"old-token");
        assert!(!registration_path.exists());
    }

    #[test]
    fn enterprise_profile_arn_uses_region_fallback() {
        assert_eq!(
            resolve_profile_arn(&KiroProvider::Enterprise, "eu-west-1", None),
            "arn:aws:codewhisperer:eu-central-1:610548660232:profile/VNECVYCYYAWN"
        );
    }
}
