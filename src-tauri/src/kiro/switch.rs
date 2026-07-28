use crate::kiro::token::RefreshedCredentials;
use crate::kiro::types::{
    KiroCredentials, KiroPoolAccount, KiroReservation, KiroSwitchOptions, KiroSwitchResult,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};

#[async_trait]
pub trait ReservationService: Send + Sync {
    async fn reserve(&self) -> Result<KiroReservation, String>;
    async fn commit(&self, id: &str, credentials: &KiroCredentials) -> Result<(), String>;
    async fn release(&self, id: &str) -> Result<(), String>;
}

#[async_trait]
pub trait TokenService: Send + Sync {
    async fn refresh(&self, account: &KiroPoolAccount) -> Result<RefreshedCredentials, String>;
}

pub trait MachineIdStore: Send + Sync {
    fn current(&self) -> Result<String, String>;
    fn set(&self, value: &str) -> Result<(), String>;
}

pub trait CredentialStore: Send + Sync {
    fn write(&self, credentials: &KiroCredentials) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
}

pub trait KiroProcess: Send + Sync {
    fn is_running(&self) -> bool;
    fn close(&self) -> Result<(), String>;
    fn launch(&self) -> Result<(), String>;
}

pub struct SwitchCoordinator<R, T, M, C, P> {
    reservations: R,
    tokens: T,
    machine_ids: M,
    credentials: C,
    process: P,
}

impl<R, T, M, C, P> SwitchCoordinator<R, T, M, C, P>
where
    R: ReservationService,
    T: TokenService,
    M: MachineIdStore,
    C: CredentialStore,
    P: KiroProcess,
{
    pub fn new(reservations: R, tokens: T, machine_ids: M, credentials: C, process: P) -> Self {
        Self {
            reservations,
            tokens,
            machine_ids,
            credentials,
            process,
        }
    }

    pub async fn switch(&self, options: KiroSwitchOptions) -> Result<KiroSwitchResult, String> {
        let reservation = self.reservations.reserve().await?;
        let reservation_id = reservation.reservation_id.clone();
        let result = self
            .apply_account(reservation.account, options, Some(&reservation_id))
            .await;
        if result.is_err() {
            let _ = self.reservations.release(&reservation_id).await;
        }
        result
    }

    pub async fn switch_owned(
        &self,
        account: KiroPoolAccount,
        options: KiroSwitchOptions,
    ) -> Result<KiroSwitchResult, String> {
        self.apply_account(account, options, None).await
    }

    async fn apply_account(
        &self,
        mut account: KiroPoolAccount,
        options: KiroSwitchOptions,
        reservation_id: Option<&str>,
    ) -> Result<KiroSwitchResult, String> {
        if self.process.is_running() {
            if !options.force_close {
                return Err("KIRO_RUNNING".to_string());
            }
            if let Err(error) = self.process.close() {
                return Err(error);
            }
        }
        let refreshed = match self.tokens.refresh(&account).await {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        let original_machine_id = match self.machine_ids.current() {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        if let Err(error) = self.machine_ids.set(&account.machine_id) {
            return Err(error);
        }
        let credentials = refreshed_credentials(&account, &refreshed);
        if let Err(error) = self.credentials.write(&credentials) {
            return Err(self.rollback_local(&original_machine_id, error));
        }
        if let Some(id) = reservation_id {
            if let Err(error) = self.reservations.commit(id, &credentials).await {
                return Err(self.rollback_local(&original_machine_id, error));
            }
        }
        account.access_token = refreshed.access_token;
        account.refresh_token = refreshed.refresh_token;
        account.expires_at = (Utc::now() + Duration::seconds(refreshed.expires_in)).timestamp();
        let launch_error = options
            .launch_after_switch
            .then(|| self.process.launch().err())
            .flatten();
        Ok(KiroSwitchResult {
            email: account.email.clone(),
            provider: account.provider.clone(),
            machine_id: account.machine_id.clone(),
            deducted_credits: if reservation_id.is_some() {
                account.credit_quota
            } else {
                0
            },
            account,
            launch_error,
            sync_error: None,
        })
    }

    fn rollback_local(&self, original_machine_id: &str, error: String) -> String {
        let mut failures = Vec::new();
        if let Err(restore_error) = self.credentials.restore() {
            failures.push(format!("restore Kiro credentials: {restore_error}"));
        }
        if let Err(restore_error) = self.machine_ids.set(original_machine_id) {
            failures.push(format!("restore MachineGuid: {restore_error}"));
        }
        if failures.is_empty() {
            error
        } else {
            format!("{error}; rollback failed: {}", failures.join("; "))
        }
    }
}

fn refreshed_credentials(
    account: &KiroPoolAccount,
    refreshed: &RefreshedCredentials,
) -> KiroCredentials {
    KiroCredentials {
        access_token: refreshed.access_token.clone(),
        refresh_token: refreshed.refresh_token.clone(),
        expires_at: (Utc::now() + Duration::seconds(refreshed.expires_in)).to_rfc3339(),
        auth_method: account.auth_method.clone(),
        provider: account.provider.clone(),
        client_id: account.client_id.clone(),
        client_secret: account.client_secret.clone(),
        region: account.region.clone(),
        start_url: account.start_url.clone(),
        profile_arn: account.profile_arn.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kiro::token::RefreshedCredentials;
    use crate::kiro::types::{
        KiroAuthMethod, KiroPoolAccount, KiroProvider, KiroReservation, KiroSwitchOptions,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct State {
        machine_id: String,
        credentials_written: bool,
        committed: bool,
        released: bool,
        launched: bool,
        reserved: bool,
    }

    struct FakeReservationService(Arc<Mutex<State>>);
    struct FakeTokenService;
    struct FakeMachineStore(Arc<Mutex<State>>);
    struct FakeCredentialStore(Arc<Mutex<State>>, bool);
    struct FakeProcess(Arc<Mutex<State>>);
    struct FakeLaunchFailureProcess;

    #[async_trait::async_trait]
    impl ReservationService for FakeReservationService {
        async fn reserve(&self) -> Result<KiroReservation, String> {
            self.0.lock().unwrap().reserved = true;
            Ok(reservation())
        }
        async fn commit(&self, _id: &str, _credentials: &KiroCredentials) -> Result<(), String> {
            self.0.lock().unwrap().committed = true;
            Ok(())
        }
        async fn release(&self, _id: &str) -> Result<(), String> {
            self.0.lock().unwrap().released = true;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl TokenService for FakeTokenService {
        async fn refresh(
            &self,
            _account: &KiroPoolAccount,
        ) -> Result<RefreshedCredentials, String> {
            Ok(RefreshedCredentials {
                access_token: "new-access".to_string(),
                refresh_token: "new-refresh".to_string(),
                expires_in: 3600,
            })
        }
    }

    impl MachineIdStore for FakeMachineStore {
        fn current(&self) -> Result<String, String> {
            Ok(self.0.lock().unwrap().machine_id.clone())
        }
        fn set(&self, value: &str) -> Result<(), String> {
            self.0.lock().unwrap().machine_id = value.to_string();
            Ok(())
        }
    }

    impl CredentialStore for FakeCredentialStore {
        fn write(&self, _credentials: &crate::kiro::types::KiroCredentials) -> Result<(), String> {
            if self.1 {
                return Err("credential write failed".to_string());
            }
            self.0.lock().unwrap().credentials_written = true;
            Ok(())
        }
        fn restore(&self) -> Result<(), String> {
            self.0.lock().unwrap().credentials_written = false;
            Ok(())
        }
    }

    impl KiroProcess for FakeProcess {
        fn is_running(&self) -> bool {
            false
        }
        fn close(&self) -> Result<(), String> {
            Ok(())
        }
        fn launch(&self) -> Result<(), String> {
            self.0.lock().unwrap().launched = true;
            Ok(())
        }
    }

    impl KiroProcess for FakeLaunchFailureProcess {
        fn is_running(&self) -> bool {
            false
        }
        fn close(&self) -> Result<(), String> {
            Ok(())
        }
        fn launch(&self) -> Result<(), String> {
            Err("launch failed".to_string())
        }
    }

    #[tokio::test]
    async fn launch_failure_after_commit_returns_success_with_warning() {
        let state = Arc::new(Mutex::new(State {
            machine_id: "11111111-1111-4111-8111-111111111111".to_string(),
            ..State::default()
        }));
        let coordinator = SwitchCoordinator::new(
            FakeReservationService(state.clone()),
            FakeTokenService,
            FakeMachineStore(state.clone()),
            FakeCredentialStore(state.clone(), false),
            FakeLaunchFailureProcess,
        );

        let result = coordinator
            .switch(KiroSwitchOptions {
                force_close: true,
                launch_after_switch: true,
            })
            .await
            .expect("the committed switch must remain successful");

        assert_eq!(result.launch_error.as_deref(), Some("launch failed"));
        assert!(state.lock().unwrap().committed);
    }

    #[tokio::test]
    async fn successful_switch_commits_and_launches() {
        let state = Arc::new(Mutex::new(State {
            machine_id: "11111111-1111-4111-8111-111111111111".to_string(),
            ..State::default()
        }));
        let coordinator = SwitchCoordinator::new(
            FakeReservationService(state.clone()),
            FakeTokenService,
            FakeMachineStore(state.clone()),
            FakeCredentialStore(state.clone(), false),
            FakeProcess(state.clone()),
        );
        let result = coordinator
            .switch(KiroSwitchOptions {
                force_close: true,
                launch_after_switch: true,
            })
            .await
            .unwrap();

        let state = state.lock().unwrap();
        assert_eq!(state.machine_id, result.machine_id);
        assert!(state.credentials_written && state.committed && state.launched);
        assert!(!state.released);
    }

    #[tokio::test]
    async fn credential_failure_restores_machine_id_and_releases_reservation() {
        let original = "11111111-1111-4111-8111-111111111111";
        let state = Arc::new(Mutex::new(State {
            machine_id: original.to_string(),
            ..State::default()
        }));
        let coordinator = SwitchCoordinator::new(
            FakeReservationService(state.clone()),
            FakeTokenService,
            FakeMachineStore(state.clone()),
            FakeCredentialStore(state.clone(), true),
            FakeProcess(state.clone()),
        );
        assert!(coordinator
            .switch(KiroSwitchOptions {
                force_close: true,
                launch_after_switch: false,
            })
            .await
            .is_err());

        let state = state.lock().unwrap();
        assert_eq!(state.machine_id, original);
        assert!(state.released);
        assert!(!state.committed);
    }

    #[tokio::test]
    async fn owned_account_switch_does_not_use_reservation_service() {
        let state = Arc::new(Mutex::new(State {
            machine_id: "11111111-1111-4111-8111-111111111111".to_string(),
            ..State::default()
        }));
        let coordinator = SwitchCoordinator::new(
            FakeReservationService(state.clone()),
            FakeTokenService,
            FakeMachineStore(state.clone()),
            FakeCredentialStore(state.clone(), false),
            FakeProcess(state.clone()),
        );
        let result = coordinator
            .switch_owned(
                reservation().account,
                KiroSwitchOptions {
                    force_close: true,
                    launch_after_switch: true,
                },
            )
            .await
            .unwrap();

        let state = state.lock().unwrap();
        assert!(!state.reserved && !state.committed && !state.released);
        assert!(state.credentials_written && state.launched);
        assert_eq!(result.deducted_credits, 0);
    }

    fn reservation() -> KiroReservation {
        KiroReservation {
            reservation_id: "reservation-id".to_string(),
            expires_at: i64::MAX,
            account: KiroPoolAccount {
                id: 1,
                email: "kiro@example.com".to_string(),
                provider: KiroProvider::BuilderId,
                auth_method: KiroAuthMethod::IdC,
                access_token: "old-access".to_string(),
                refresh_token: "old-refresh".to_string(),
                expires_at: 0,
                client_id: Some("client-id".to_string()),
                client_secret: Some("client-secret".to_string()),
                region: "us-east-1".to_string(),
                start_url: Some("https://view.awsapps.com/start".to_string()),
                profile_arn: None,
                machine_id: "22222222-2222-4222-8222-222222222222".to_string(),
                credit_quota: 50,
            },
        }
    }
}
