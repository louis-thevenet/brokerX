/// Centralized MFA Service
/// 
/// This module provides a clean, professional API for MFA operations in the API Gateway.
/// It encapsulates all MFA logic and provides clear, REST-compliant methods.

use crate::error::{ApiError, ApiResult};
use mfa_adapter::{EmailOtpProvider, MfaError, MfaProvider};
use std::sync::Arc;

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
        self.provider
            .send_otp(email)
            .await
            .map_err(Self::map_mfa_error)
    }

    /// Verify an MFA challenge with the provided code
    /// 
    /// # Returns
    /// - Ok(user_email) if verification is successful
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
            MfaError::ChallengeExpired => {
                ApiError::BadRequest("MFA challenge has expired. Please request a new code.".to_string())
            }
            MfaError::InvalidCode => {
                ApiError::BadRequest("Invalid verification code".to_string())
            }
            MfaError::SendingFailed(msg) => {
                ApiError::InternalServerError(format!("Failed to send verification code: {}", msg))
            }
            MfaError::ServiceUnavailable => {
                ApiError::ServiceUnavailable("MFA service is temporarily unavailable".to_string())
            }
        }
    }
}
