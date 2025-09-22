use crate::mfa::MfaService;
use mfa_adapter::EmailOtpProvider;

/// Factory for creating MFA services with different providers
pub struct MfaServiceFactory;

impl MfaServiceFactory {
    /// Creates an email-based MFA service with default configuration
    pub fn create_email_mfa_service() -> MfaService<EmailOtpProvider> {
        let email_provider = EmailOtpProvider::new_with_default_config();
        MfaService::new(email_provider)
    }
}

/// Type alias for the default MFA service implementation
pub type DefaultMfaService = MfaService<EmailOtpProvider>;
