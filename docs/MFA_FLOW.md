# MFA Flow - Refactored and Clarified

## Overview

The MFA (Multi-Factor Authentication) logic has been completely refactored to follow REST API best practices and provide a clean, professional authentication flow.

## Key Changes

### 1. **Centralized MFA Service**
- Created `mfa_service.rs` module to encapsulate all MFA logic
- Single source of truth for MFA operations
- Clean error handling and mapping to HTTP status codes

### 2. **Simplified API Endpoints**

#### Old Endpoints (Removed):
- ❌ `/auth/initiate-registration-mfa` 
- ❌ `/auth/verify-registration`
- ❌ `/me/initiate-email-verification`
- ❌ `/me/verify-email`

#### New Endpoints:
- ✅ `/auth/register` - Automatically sends MFA challenge
- ✅ `/auth/login` - Returns token OR challenge_id
- ✅ `/auth/verify-login` - Complete MFA verification

### 3. **Clean Authentication Flow**

## User Flows

### Registration Flow

```
1. User submits registration form
   POST /auth/register
   {
     "email": "user@example.com",
     "password": "SecurePassword123!",
     "firstname": "John",
     "surname": "Doe"
   }

2. Backend:
   - Creates user account (is_verified=false)
   - Does NOT send MFA email yet
   - Returns success response
   
   Response (201):
   {
     "user_id": "uuid",
     "message": "Registration successful. Please login to verify your email...",
     "requires_verification": true
   }

3. User attempts first login with new credentials
   POST /auth/login
   {
     "email": "user@example.com",
     "password": "SecurePassword123!"
   }

4. Backend:
   - Authenticates credentials (user-service checks email/password only)
   - Checks is_verified flag → FALSE
   - Sends MFA code to email (FIRST TIME)
   - Returns challenge_id
   
   Response (200):
   {
     "status": "requires_verification",
     "user_id": "uuid",
     "challenge_id": "challenge-uuid",
     "message": "Email verification required. Check your email for the 6-digit code."
   }

5. User receives email with 6-digit code and enters it
   POST /auth/verify-login
   {
     "challenge_id": "challenge-uuid",
     "code": "123456"
   }

6. Backend:
   - Verifies MFA code
   - Marks user as verified (is_verified=true)
   - Issues JWT token
   
   Response (200):
   {
     "token": "jwt-token",
     "user_id": "uuid",
     "expires_at": "2025-10-14T12:00:00Z",
     "message": "Email verified successfully. You are now logged in."
   }

7. User is now verified and logged in
```

### Login Flow (Already Verified User)

```
1. User submits login form
   POST /auth/login
   {
     "email": "user@example.com",
     "password": "SecurePassword123!"
   }

2. Backend checks if user is verified:
   - User IS verified → Returns JWT token immediately
   
   Response (200):
   {
     "status": "success",
     "token": "jwt-token",
     "user_id": "uuid",
     "expires_at": "2025-10-14T12:00:00Z"
   }

3. User is logged in - no MFA required
```

### Login Flow (Unverified User)

```
1. User submits login form
   POST /auth/login

2. Backend:
   - Authenticates credentials (user-service validates email/password)
   - Detects is_verified=false
   - Sends MFA code to email
   - Returns challenge_id
   
   Response (200):
   {
     "status": "requires_verification",
     "user_id": "uuid",
     "challenge_id": "challenge-uuid",
     "message": "Email verification required. Check your email for the code."
   }

3. User enters MFA code from email
   POST /auth/verify-login
   {
     "challenge_id": "challenge-uuid",
     "code": "123456"
   }

4. Backend:
   - Verifies code
   - Marks user as verified
   - Issues JWT token

5. User is now verified and logged in
```

## API Models

### LoginResponse (Enum)
```rust
pub enum LoginResponse {
    Success {
        token: String,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    },
    RequiresVerification {
        user_id: Uuid,
        challenge_id: String,
        message: String,
    },
}
```

### VerifyMfaRequest
```rust
pub struct VerifyMfaRequest {
    pub challenge_id: String,
    pub code: String,
}
```

### VerifyMfaResponse
```rust
pub struct VerifyMfaResponse {
    pub token: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub message: String,
}
```

## Benefits of New Design

### 1. **REST Compliance**
- Clear, predictable endpoints
- Proper HTTP status codes
- Stateless authentication

### 2. **Security**
- MFA enforced before token issuance
- No token issued to unverified users
- Challenge IDs prevent replay attacks

### 3. **User Experience**
- Simplified flow: Register → Login → Verify
- Clear error messages
- Automatic MFA code sending

### 4. **Maintainability**
- Centralized MFA logic in `mfa_service.rs`
- Single verification endpoint
- Clear separation of concerns

### 5. **Professional Standards**
- Follows OAuth2/OIDC patterns
- Compatible with financial security regulations
- Production-ready error handling

## Implementation Details

### MFA Service (`mfa_service.rs`)
```rust
pub struct MfaService {
    provider: Arc<EmailOtpProvider>,
}

impl MfaService {
    pub async fn initiate_challenge(&self, email: &str) -> ApiResult<String>
    pub async fn verify_challenge(&self, challenge_id: &str, code: &str) -> ApiResult<String>
    pub fn get_challenge_email(&self, challenge_id: &str) -> ApiResult<String>
}
```

### Error Handling
- `ChallengeNotFound` → 400 Bad Request
- `ChallengeExpired` → 400 Bad Request  
- `InvalidCode` → 400 Bad Request
- `SendingFailed` → 500 Internal Server Error
- `ServiceUnavailable` → 503 Service Unavailable

## Testing

### Manual Testing Flow
1. Register new user → Check email for code
2. Login with new user → Should get challenge_id
3. Verify with code → Should get token
4. Login again → Should get token immediately (no MFA)

### WebApp Testing
1. Open `http://localhost:8080`
2. Register new account
3. Check browser console for API calls
4. Verify MFA flow works end-to-end

## Migration Notes

### For Frontend Developers
- Remove calls to old MFA endpoints
- Handle `LoginResponse` enum with two variants
- Store challenge_id when login returns `requires_verification`
- Call `/auth/verify-login` with challenge_id and code

### For Backend Developers
- MFA logic is now in `mfa_service.rs`
- All MFA operations go through `MfaService`
- Login endpoint checks `is_verified` flag
- Single verification endpoint handles all cases

## Future Enhancements

1. **Rate Limiting**: Add rate limiting to MFA endpoints
2. **Challenge Expiry**: Configurable challenge expiration times
3. **SMS/TOTP**: Support additional MFA methods
4. **Recovery Codes**: Backup codes for account recovery
5. **MFA Settings**: Allow users to configure MFA preferences

## Related Files

- `api_gateway/src/mfa_service.rs` - MFA service implementation
- `api_gateway/src/api.rs` - API endpoints
- `api_gateway/src/models.rs` - Request/response models
- `webapp/app.js` - Frontend implementation
- `mfa_adapter/` - MFA provider implementation
