/// Centralized MFA Service
///
/// This module provides a clean, professional API for MFA operations in the API Gateway.
/// It encapsulates all MFA logic and provides clear, REST-compliant methods.
use crate::error::{ApiError, ApiResult};
use mfa_adapter::{EmailOtpProvider, MfaError, MfaProvider};
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

/// MFA Service for handling all MFA-related operations
#[derive(Clone)]
pub struct MfaService {
    provider: Arc<EmailOtpProvider>,
}

impl MfaService {
    pub fn new(provider: Arc<EmailOtpProvider>) -> Self {
        Self { provider }
    }

    /// Initiate an MFA challenge by sending an OTP to the user's email
    ///
    /// # Returns
    /// The challenge ID that must be used to verify the OTP
    pub async fn initiate_challenge(&self, email: &str) -> ApiResult<String> {
        let challenge_id = Uuid::new_v4().to_string();
        self.provider
            .generate_otp(challenge_id.clone(), email)
            .await
            .map_err(Self::map_mfa_error)?;

        Ok(challenge_id)
    }

    /// Verify an MFA challenge with the provided code
    ///
    /// # Returns
    /// - `Ok(user_email)` if verification is successful
    /// - Err if the code is invalid, challenge not found, or expired
    pub async fn verify_challenge(&self, challenge_id: &str, code: &str) -> ApiResult<String> {
        // First, verify the code is valid
        let is_valid = self
            .provider
            .verify_otp(challenge_id, code)
            .map_err(Self::map_mfa_error)?;

        if !is_valid {
            return Err(ApiError::BadRequest(
                "Invalid verification code".to_string(),
            ));
        }

        // Get the challenge to extract user information
        let challenge = self
            .provider
            .get_challenge(challenge_id)
            .map_err(Self::map_mfa_error)?;

        Ok(challenge.user_email)
    }

    /// Initiate an MFA challenge without sending an email (for load testing)
    /// # Returns
    /// The challenge ID that must be used to verify the OTP
    pub async fn initiate_challenge_no_email(&self, email: &str) -> ApiResult<String> {
        let challenge_id = Uuid::new_v4().to_string();
        self.provider
            .generate_otp_no_email(challenge_id.clone(), email)
            .map_err(Self::map_mfa_error)?;

        ApiResult::Ok(challenge_id)
    }

    /// Verify an MFA challenge in bypass mode (for load testing)
    ///
    /// This method accepts any code when MFA_BYPASS is enabled and returns the user's email.
    ///
    /// # Returns
    /// - `Ok(user_email)` if the challenge exists and is not expired
    /// - Err if challenge not found or expired
    pub async fn verify_challenge_bypass(&self, challenge_id: &str) -> ApiResult<String> {
        // Get the challenge to extract user information
        // We don't verify the code, just check if the challenge exists and is valid
        let challenge = self
            .provider
            .get_challenge(challenge_id)
            .map_err(Self::map_mfa_error)?;

        // Check if challenge has expired
        let now = SystemTime::now();
        if now > challenge.expires_at {
            return Err(ApiError::BadRequest(
                "MFA challenge has expired. Please request a new code.".to_string(),
            ));
        }

        Ok(challenge.user_email)
    }

    /// Get challenge information
    pub fn get_challenge_email(&self, challenge_id: &str) -> ApiResult<String> {
        let challenge = self
            .provider
            .get_challenge(challenge_id)
            .map_err(Self::map_mfa_error)?;

        Ok(challenge.user_email)
    }

    /// Map MFA errors to API errors with appropriate HTTP status codes
    fn map_mfa_error(error: MfaError) -> ApiError {
        match error {
            MfaError::ChallengeNotFound => {
                ApiError::BadRequest("MFA challenge not found".to_string())
            }
            MfaError::ChallengeExpired => ApiError::BadRequest(
                "MFA challenge has expired. Please request a new code.".to_string(),
            ),
            MfaError::InvalidCode => ApiError::BadRequest("Invalid verification code".to_string()),
            MfaError::SendingFailed(msg) => {
                ApiError::InternalServerError(format!("Failed to send verification code: {msg}"))
            }
            MfaError::ServiceUnavailable => {
                ApiError::ServiceUnavailable("MFA service is temporarily unavailable".to_string())
            }
        }
    }
}
