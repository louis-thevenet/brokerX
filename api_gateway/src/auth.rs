use crate::error::{ApiError, ApiResult};
use crate::models::Claims;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiry_hours: i64,
}

impl JwtService {
    pub fn new(secret: &str, expiry_hours: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            expiry_hours,
        }
    }

    pub fn create_token(
        &self,
        user_id: Uuid,
    ) -> ApiResult<(String, chrono::DateTime<chrono::Utc>)> {
        let now = Utc::now();
        let expires_at = now + Duration::hours(self.expiry_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expires_at.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok((token, expires_at))
    }

    pub fn validate_token(&self, token: &str) -> ApiResult<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &Validation::default())?;
        if token_data.claims.exp < Utc::now().timestamp() as usize {
            return Err(ApiError::Unauthorized);
        }
        Ok(token_data.claims)
    }

    pub fn extract_user_id(&self, token: &str) -> ApiResult<Uuid> {
        let claims = self.validate_token(token)?;
        Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)
    }
}
