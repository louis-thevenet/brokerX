use color_eyre::Result;

// Re-export types from mfa_adapter for domain use
pub use mfa_adapter::{MfaError, MfaProvider, OtpChallenge};

/// Service for managing MFA operations
#[derive(Debug)]
pub struct MfaService<P: MfaProvider> {
    provider: P,
}

impl<P: MfaProvider> MfaService<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Initiates MFA for a user by sending OTP
    pub async fn initiate_mfa(&self, user_email: &str) -> Result<String, MfaError> {
        self.provider.send_otp(user_email).await
    }

    /// Verifies MFA challenge
    pub async fn verify_mfa(&self, challenge_id: &str, code: &str) -> Result<bool, MfaError> {
        self.provider.verify_otp(challenge_id, code).await
    }

    /// Gets challenge information
    pub async fn get_challenge(&self, challenge_id: &str) -> Result<OtpChallenge, MfaError> {
        self.provider.get_challenge(challenge_id).await
    }
}
