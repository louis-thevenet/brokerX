# Bug Fix: User Email Update Not Working

**Date**: October 13, 2025  
**Issue**: Email field was not being updated when calling the `/me` endpoint  
**Status**: ✅ **FIXED**

---

## Problem Description

When attempting to update a user's email via the web application, the email field was always showing as `None` in the user service logs, even though the frontend was sending the email value correctly.

### Observed Behavior
```
2025-10-13T15:48:13.905452Z  INFO user_service::api: Updating user with ID: 8f3bfaad-a8cb-43e8-a54b-f5db8f3348fa
2025-10-13T15:48:13.905547Z DEBUG user_service::api: UpdateUserRequest { email: None, firstname: None, surname: None }
```

The frontend was correctly sending the email in the request body, but it was being dropped somewhere in the pipeline.

---

## Root Cause

The `UpdateUserRequest` model in the API gateway (`api_gateway/src/models.rs`) was missing the `email` field:

```rust
// BEFORE (Bug)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub firstname: Option<String>,  // ✅ Present
    pub surname: Option<String>,    // ✅ Present
    // ❌ email field was MISSING!
}
```

This meant that when the API gateway received the update request from the frontend, it would deserialize the JSON and **silently drop the email field** because it wasn't part of the struct definition. The gateway would then forward the request to the user service with `email: None`.

---

## Solution

Added the `email` field to the `UpdateUserRequest` model in the API gateway:

```rust
// AFTER (Fixed)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,      // ✅ Added
    pub firstname: Option<String>,
    pub surname: Option<String>,
}
```

**File Changed**: `/home/louis/src/brokerx/api_gateway/src/models.rs`

---

## Verification

### Compilation Check
```bash
cd /home/louis/src/brokerx
cargo check -p api_gateway
# ✅ Success - Finished `dev` profile [unoptimized + debuginfo] target(s)
```

### Expected Behavior After Fix

1. **Frontend sends**:
   ```json
   {
     "email": "newemail@example.com"
   }
   ```

2. **API Gateway receives and forwards**:
   ```json
   {
     "email": "newemail@example.com",
     "firstname": null,
     "surname": null
   }
   ```

3. **User Service logs**:
   ```
   DEBUG user_service::api: UpdateUserRequest { email: Some("newemail@example.com"), firstname: None, surname: None }
   ```

4. **User's email is updated in the database** ✅

---

## Testing Steps

1. **Start the services**:
   ```bash
   ./start-all.sh
   ```

2. **Open the web application**: http://localhost:8000

3. **Login to your account**

4. **Go to Profile section**

5. **Update email**:
   - Enter a new email address in the "New Email" field
   - Leave other fields blank
   - Click "Update Profile"

6. **Verify in logs**:
   - Check user-service logs for `UpdateUserRequest`
   - Should show: `email: Some("new@email.com")`

7. **Verify in UI**:
   - Check that the email is updated in the profile display
   - Logout and login again to confirm persistence

---

## Related Files

- **Fixed**: `/home/louis/src/brokerx/api_gateway/src/models.rs`
- **Frontend**: `/home/louis/src/brokerx/webapp/app.js` (no changes needed, was already correct)
- **User Service**: `/home/louis/src/brokerx/user-service/src/repository.rs` (no changes needed, was already correct)

---

## Impact

- **Severity**: Medium (feature not working as intended)
- **Affected Users**: All users trying to update their email
- **Fix Complexity**: Trivial (one line addition)
- **Regression Risk**: Very low (only adds a field, doesn't change existing behavior)

---

## Lessons Learned

1. **Always check the full request/response chain**: The bug was in the middle layer (API gateway), not the frontend or backend service.

2. **Serde silently drops unknown fields by default**: When deserializing, fields in the JSON that don't exist in the struct are ignored without error.

3. **Model consistency is critical**: The API gateway's request models must match what the frontend sends and what the backend expects.

4. **Better testing needed**: Integration tests or end-to-end tests would have caught this issue earlier.

---

## Future Improvements

1. **Add Integration Tests**: Test the full flow from API gateway to user service
   ```rust
   #[tokio::test]
   async fn test_update_user_email() {
       // Test that email updates work end-to-end
   }
   ```

2. **API Contract Validation**: Use OpenAPI/Swagger to validate request/response models across services

3. **Add Comprehensive Logging**: Log the raw request body at each layer to help debug issues like this

4. **Type-safe Service Communication**: Consider using gRPC or similar for service-to-service communication to catch model mismatches at compile time

---

**Status**: ✅ Fixed and ready for testing  
**Deployed**: Requires service restart to take effect
