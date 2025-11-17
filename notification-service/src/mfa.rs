// Legacy MFA support for backward compatibility
// This module provides the original MFA interface while using the new notification system internally

use crate::notification_models::Notification;
use crate::providers::EmailProvider;
use crate::service::NotificationService;
use color_eyre::Result;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tracing::debug;
use uuid::Uuid;

pub use crate::providers::email::EmailConfig;

#[derive(Debug, Clone)]
pub enum MfaError {
    SendingFailed(String),
    ChallengeNotFound,
    ChallengeExpired,
    InvalidCode,
    ServiceUnavailable,
}

impl std::fmt::Display for MfaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MfaError::SendingFailed(msg) => write!(f, "Failed to send OTP: {msg}"),
            MfaError::ChallengeNotFound => write!(f, "Challenge not found"),
            MfaError::ChallengeExpired => write!(f, "Challenge has expired"),
            MfaError::InvalidCode => write!(f, "Invalid verification code"),
            MfaError::ServiceUnavailable => write!(f, "MFA service is temporarily unavailable"),
        }
    }
}

impl std::error::Error for MfaError {}

// OTP Challenge structure (kept for compatibility)
#[derive(Debug, Clone)]
pub struct OtpChallenge {
    pub id: String,
    pub user_email: String,
    pub code: String,
    pub verified: bool,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
}

pub trait MfaProvider: Send + Sync {
    fn generate_otp(
        &self,
        challenge_id: String,
        user_email: &str,
    ) -> impl std::future::Future<Output = Result<String, MfaError>> + Send;
    fn generate_otp_no_email(
        &self,
        challenge_id: String,
        user_email: &str,
    ) -> Result<String, MfaError>;
    fn verify_otp(&self, challenge_id: &str, code: &str) -> Result<bool, MfaError>;
    fn get_challenge(&self, challenge_id: &str) -> Result<OtpChallenge, MfaError>;

    fn generate_challenge(
        &self,
        challenge_id: String,
        user_email: &str,
    ) -> Result<OtpChallenge, MfaError>;
}

/// Legacy EmailOtpProvider that wraps the new notification system
pub struct EmailOtpProvider {
    notification_service: Arc<NotificationService>,
    challenges: Arc<Mutex<HashMap<String, OtpChallenge>>>,
    challenge_duration: Duration,
}

impl EmailOtpProvider {
    #[must_use]
    pub fn new(config: EmailConfig) -> Self {
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            crate::notification_models::DeliveryChannel::Email,
            Arc::new(EmailProvider::new(config)) as Arc<dyn crate::providers::NotificationProvider>,
        );

        let notification_service = Arc::new(NotificationService::with_providers(providers));

        Self {
            notification_service,
            challenges: Arc::new(Mutex::new(HashMap::new())),
            challenge_duration: Duration::from_secs(300), // 5 minutes
        }
    }

    pub fn new_with_default_config() -> Self {
        let notification_service = Arc::new(NotificationService::new());

        Self {
            notification_service,
            challenges: Arc::new(Mutex::new(HashMap::new())),
            challenge_duration: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create `EmailOtpProvider` without email configuration (console logging only)
    #[must_use]
    pub fn new_without_email() -> Self {
        let mut providers = std::collections::HashMap::new();
        providers.insert(
            crate::notification_models::DeliveryChannel::Email,
            Arc::new(EmailProvider::new_console_fallback())
                as Arc<dyn crate::providers::NotificationProvider>,
        );

        let notification_service = Arc::new(NotificationService::with_providers(providers));

        Self {
            notification_service,
            challenges: Arc::new(Mutex::new(HashMap::new())),
            challenge_duration: Duration::from_secs(300),
        }
    }

    /// Create `EmailOtpProvider` with test configuration for testing purposes
    #[must_use]
    pub fn new_for_testing() -> Self {
        Self::new(EmailConfig::new_test_config())
    }

    /// Create `EmailOtpProvider` with test configuration and custom challenge duration
    #[cfg(test)]
    #[must_use]
    pub fn new_for_testing_with_duration(duration: Duration) -> Self {
        let mut provider = Self::new(EmailConfig::new_test_config());
        provider.challenge_duration = duration;
        provider
    }

    /// Create `EmailOtpProvider` with configuration from environment variables
    pub fn new_from_env() -> Result<Self, String> {
        let config = EmailConfig::from_env()?;
        Ok(Self::new(config))
    }

    fn generate_otp_code(&self) -> String {
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(100_000..999_999))
    }

    async fn send_mfa_notification(
        &self,
        user_email: &str,
        challenge_id: &str,
        code: &str,
    ) -> Result<(), MfaError> {
        // Create a fake user ID for MFA notifications (in real system, you'd get this from somewhere)
        let user_id = Uuid::new_v4().to_string();

        let notification = Notification::mfa(
            user_id,
            user_email.to_string(),
            challenge_id.to_string(),
            code.to_string(),
        );

        let results = self
            .notification_service
            .send_notification(notification)
            .await
            .map_err(|e| MfaError::SendingFailed(e.to_string()))?;

        // Check if at least one channel succeeded
        let success = results.iter().any(|result| result.success);
        if success {
            Ok(())
        } else {
            Err(MfaError::SendingFailed(
                "Failed to send notification via any channel".to_string(),
            ))
        }
    }
}

impl MfaProvider for EmailOtpProvider {
    async fn generate_otp(
        &self,
        challenge_id: String,
        user_email: &str,
    ) -> Result<String, MfaError> {
        let challenge = self.generate_challenge(challenge_id.clone(), user_email)?;
        let code = challenge.code.clone();
        self.send_mfa_notification(user_email, &challenge_id, &code)
            .await?;
        Ok(code)
    }

    fn generate_otp_no_email(
        &self,
        challenge_id: String,
        user_email: &str,
    ) -> Result<String, MfaError> {
        let challenge = self.generate_challenge(challenge_id.clone(), user_email)?;
        Ok(challenge.code)
    }

    fn generate_challenge(
        &self,
        challenge_id: String,
        user_email: &str,
    ) -> Result<OtpChallenge, MfaError> {
        let code = self.generate_otp_code();
        debug!(
            "Generated OTP code: {} for challenge ID: {}",
            code, challenge_id
        );
        let now = SystemTime::now();
        let expires_at = now + self.challenge_duration;

        // Store the challenge with the user's actual email for identification
        let challenge = OtpChallenge {
            id: challenge_id.clone(),
            user_email: user_email.to_string(),
            code,
            verified: false,
            created_at: now,
            expires_at,
        };

        let mut challenges = self.challenges.lock().unwrap();
        challenges.insert(challenge_id.clone(), challenge.clone());

        Ok(challenge)
    }

    fn verify_otp(&self, challenge_id: &str, code: &str) -> Result<bool, MfaError> {
        let mut challenges = self.challenges.lock().unwrap();

        let challenge = challenges
            .get_mut(challenge_id)
            .ok_or(MfaError::ChallengeNotFound)?;

        // Check if challenge has expired
        if SystemTime::now() > challenge.expires_at {
            challenges.remove(challenge_id);
            return Err(MfaError::ChallengeExpired);
        }

        // Check if already verified
        if challenge.verified {
            return Ok(true);
        }

        // Verify the code
        if challenge.code == code {
            challenge.verified = true;
            Ok(true)
        } else {
            Err(MfaError::InvalidCode)
        }
    }

    fn get_challenge(&self, challenge_id: &str) -> Result<OtpChallenge, MfaError> {
        let challenges = self.challenges.lock().unwrap();

        let challenge = challenges
            .get(challenge_id)
            .ok_or(MfaError::ChallengeNotFound)?;

        // Check if challenge has expired
        if SystemTime::now() > challenge.expires_at {
            return Err(MfaError::ChallengeExpired);
        }

        Ok(challenge.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_otp_code() {
        let provider = EmailOtpProvider::new_for_testing();
        let code = provider.generate_otp_code();

        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));

        let code_num: u32 = code.parse().unwrap();
        assert!((100_000..1_000_000).contains(&code_num));
    }

    #[test]
    fn test_challenge_expiry() {
        // Create a provider with very short expiry duration for testing
        let provider = EmailOtpProvider::new_for_testing_with_duration(Duration::from_millis(10));

        // Create a mock challenge directly for testing expiry logic
        let challenge_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();
        let challenge = OtpChallenge {
            id: challenge_id.clone(),
            user_email: "test@example.com".to_string(),
            code: "123456".to_string(),
            verified: false,
            created_at: now,
            expires_at: now + Duration::from_millis(10),
        };

        // Manually insert the challenge
        {
            let mut challenges = provider.challenges.lock().unwrap();
            challenges.insert(challenge_id.clone(), challenge);
        }

        // Wait for the challenge to expire
        std::thread::sleep(Duration::from_millis(20));

        // Try to get the expired challenge
        let result = provider.get_challenge(&challenge_id);
        assert!(matches!(result, Err(MfaError::ChallengeExpired)));
    }
}
