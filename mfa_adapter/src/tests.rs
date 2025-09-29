use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::{MfaError, MfaProvider, OtpChallenge};

#[derive(Debug)]
pub struct MockMfaProvider {
    challenges: Arc<Mutex<HashMap<String, OtpChallenge>>>,
    should_fail_send: bool,
    should_fail_verify: bool,
}

impl MockMfaProvider {
    pub fn new() -> Self {
        Self {
            challenges: Arc::new(Mutex::new(HashMap::new())),
            should_fail_send: false,
            should_fail_verify: false,
        }
    }

    pub fn with_send_failure(mut self) -> Self {
        self.should_fail_send = true;
        self
    }

    pub fn with_verify_failure(mut self) -> Self {
        self.should_fail_verify = true;
        self
    }

    pub fn get_last_challenge(&self) -> Option<OtpChallenge> {
        let challenges = self.challenges.lock().unwrap();
        challenges.values().last().cloned()
    }
}

impl Default for MockMfaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MfaProvider for MockMfaProvider {
    async fn send_otp(&self, user_email: &str) -> Result<String, MfaError> {
        if self.should_fail_send {
            return Err(MfaError::SendingFailed("Mock failure".into()));
        }

        let challenge_id = uuid::Uuid::new_v4().to_string();
        let code = "123456".to_string(); // Fixed code for testing
        let now = SystemTime::now();

        let challenge = OtpChallenge {
            id: challenge_id.clone(),
            user_email: user_email.to_string(),
            code,
            verified: false,
            created_at: now,
            expires_at: now + Duration::from_secs(300),
        };

        self.challenges
            .lock()
            .unwrap()
            .insert(challenge_id.clone(), challenge);
        Ok(challenge_id)
    }

    fn verify_otp(&self, challenge_id: &str, code: &str) -> Result<bool, MfaError> {
        if self.should_fail_verify {
            return Err(MfaError::ServiceUnavailable);
        }

        let mut challenges = self.challenges.lock().unwrap();

        let challenge = challenges
            .get_mut(challenge_id)
            .ok_or(MfaError::ChallengeNotFound)?;

        if SystemTime::now() > challenge.expires_at {
            return Err(MfaError::ChallengeExpired);
        }

        if challenge.code != code {
            return Err(MfaError::InvalidCode);
        }

        challenge.verified = true;
        Ok(true)
    }

    fn get_challenge(&self, challenge_id: &str) -> Result<OtpChallenge, MfaError> {
        let challenges = self.challenges.lock().unwrap();
        challenges
            .get(challenge_id)
            .cloned()
            .ok_or(MfaError::ChallengeNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mfa::MfaService;

    #[tokio::test]
    async fn initiate_mfa_creates_challenge() {
        let provider = MockMfaProvider::new();
        let service = MfaService::new(provider);

        let challenge_id = service.initiate_mfa("test@example.com").await.unwrap();
        assert!(!challenge_id.is_empty());
    }

    #[tokio::test]
    async fn initiate_mfa_handles_send_failure() {
        let provider = MockMfaProvider::new().with_send_failure();
        let service = MfaService::new(provider);

        let result = service.initiate_mfa("test@example.com").await;
        assert!(matches!(result, Err(MfaError::SendingFailed(_))));
    }

    #[test]
    fn verify_mfa_succeeds_with_correct_code() {
        let provider = MockMfaProvider::new();
        let service = MfaService::new(provider);

        // First create a challenge
        let rt = tokio::runtime::Runtime::new().unwrap();
        let challenge_id = rt
            .block_on(service.initiate_mfa("test@example.com"))
            .unwrap();

        // Then verify with correct code
        let result = service.verify_mfa(&challenge_id, "123456").unwrap();
        assert!(result);
    }

    #[test]
    fn verify_mfa_fails_with_wrong_code() {
        let provider = MockMfaProvider::new();
        let service = MfaService::new(provider);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let challenge_id = rt
            .block_on(service.initiate_mfa("test@example.com"))
            .unwrap();

        let result = service.verify_mfa(&challenge_id, "wrong").unwrap_err();
        assert!(matches!(result, MfaError::InvalidCode));
    }

    #[test]
    fn verify_mfa_fails_with_missing_challenge() {
        let provider = MockMfaProvider::new();
        let service = MfaService::new(provider);

        let result = service.verify_mfa("nonexistent", "123456").unwrap_err();
        assert!(matches!(result, MfaError::ChallengeNotFound));
    }

    #[test]
    fn verify_mfa_handles_service_failure() {
        let provider = MockMfaProvider::new().with_verify_failure();
        let service = MfaService::new(provider);

        let result = service.verify_mfa("any", "123456").unwrap_err();
        assert!(matches!(result, MfaError::ServiceUnavailable));
    }

    #[test]
    fn get_challenge_returns_challenge_info() {
        let provider = MockMfaProvider::new();
        let service = MfaService::new(provider);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let challenge_id = rt
            .block_on(service.initiate_mfa("test@example.com"))
            .unwrap();

        let challenge = service.get_challenge(&challenge_id).unwrap();
        assert_eq!(challenge.user_email, "test@example.com");
        assert_eq!(challenge.code, "123456");
        assert!(!challenge.verified);
    }

    #[test]
    fn get_challenge_fails_for_missing_challenge() {
        let provider = MockMfaProvider::new();
        let service = MfaService::new(provider);

        let result = service.get_challenge("nonexistent").unwrap_err();
        assert!(matches!(result, MfaError::ChallengeNotFound));
    }

    #[test]
    fn challenge_expires_after_time() {
        let provider = MockMfaProvider::new();

        // Create an expired challenge manually
        let challenge_id = uuid::Uuid::new_v4().to_string();
        let past_time = SystemTime::now() - Duration::from_secs(3600);
        let challenge = OtpChallenge {
            id: challenge_id.clone(),
            user_email: "test@example.com".to_string(),
            code: "123456".to_string(),
            verified: false,
            created_at: past_time,
            expires_at: past_time + Duration::from_secs(300),
        };

        provider
            .challenges
            .lock()
            .unwrap()
            .insert(challenge_id.clone(), challenge);

        let result = provider.verify_otp(&challenge_id, "123456").unwrap_err();
        assert!(matches!(result, MfaError::ChallengeExpired));
    }

    #[test]
    fn challenge_verification_updates_status() {
        let provider = MockMfaProvider::new();
        let service = MfaService::new(provider);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let challenge_id = rt
            .block_on(service.initiate_mfa("test@example.com"))
            .unwrap();

        // Before verification
        let challenge = service.get_challenge(&challenge_id).unwrap();
        assert!(!challenge.verified);

        // Verify
        service.verify_mfa(&challenge_id, "123456").unwrap();

        // After verification
        let challenge = service.get_challenge(&challenge_id).unwrap();
        assert!(challenge.verified);
    }

    #[test]
    fn mock_provider_creates_valid_challenges() {
        let provider = MockMfaProvider::new();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let challenge_id = rt.block_on(provider.send_otp("user@test.com")).unwrap();

        let challenge = provider.get_challenge(&challenge_id).unwrap();
        assert_eq!(challenge.user_email, "user@test.com");
        assert_eq!(challenge.code, "123456");
        assert!(!challenge.verified);
        assert!(challenge.expires_at > challenge.created_at);
    }

    #[test]
    fn mock_provider_get_last_challenge_works() {
        let provider = MockMfaProvider::new();

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(provider.send_otp("first@test.com")).unwrap();
        rt.block_on(provider.send_otp("second@test.com")).unwrap();

        let last_challenge = provider.get_last_challenge().unwrap();
        assert_eq!(last_challenge.user_email, "second@test.com");
    }
}
