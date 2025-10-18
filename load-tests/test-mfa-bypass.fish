#!/usr/bin/env fish

# Test MFA Bypass functionality
# This script verifies that MFA bypass mode is working correctly

set -l API_BASE "http://localhost:8080"
set -l TEST_EMAIL "john.doe@example.com"
set -l TEST_PASSWORD "secure_password123"

echo "🧪 Testing MFA Bypass Functionality"
echo "===================================="
echo ""

# Step 1: Check if API Gateway is running
echo "📡 Step 1: Checking API Gateway..."
if not curl -s -f "$API_BASE/v1/health" > /dev/null
    echo "❌ ERROR: API Gateway is not running at $API_BASE"
    echo "   Start services with: docker-compose -f docker-compose.yml -f docker-compose.loadtest.yml up -d"
    exit 1
end
echo "✅ API Gateway is running"
echo ""

# Step 2: Register test user (or re-register if exists)
echo "👤 Step 2: Registering test user..."
set -l register_response (curl -s -X POST "$API_BASE/v1/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\",\"firstname\":\"John\",\"surname\":\"Doe\"}")

set -l register_user_id (echo $register_response | jq -r '.user_id // empty')
set -l register_message (echo $register_response | jq -r '.message // .error // "unknown"')

if test -n "$register_user_id"
    echo "✅ User registered successfully (ID: $register_user_id)"
else if string match -q "*already exists*" "$register_message"
    echo "ℹ️  User already exists (this is fine for testing)"
else if test "$register_message" = "unknown"
    echo "⚠️  Unexpected registration response: $register_response"
else
    echo "⚠️  Registration response: $register_message"
end
echo ""

# Step 3: Check if MFA_BYPASS is enabled
echo "🔒 Step 3: Checking MFA_BYPASS status..."
set -l bypass_check (docker logs brokerx-api-gateway 2>&1 | grep -i "mfa_bypass" | tail -1)
if test -z "$bypass_check"
    echo "⚠️  WARNING: Cannot confirm MFA_BYPASS status from logs"
    echo "   Make sure MFA_BYPASS=true is set in docker-compose"
else
    echo "✅ MFA_BYPASS appears to be configured"
    echo "   Log: $bypass_check"
end
echo ""

# Step 4: Test login (get challenge_id)
echo "🔐 Step 4: Testing login..."
set -l login_response (curl -s -X POST "$API_BASE/v1/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$TEST_EMAIL\",\"password\":\"$TEST_PASSWORD\"}")

if test $status -ne 0
    echo "❌ ERROR: Failed to connect to login endpoint"
    exit 1
end

set -l challenge_id (echo $login_response | jq -r '.challenge_id')
if test "$challenge_id" = "null" -o -z "$challenge_id"
    echo "❌ ERROR: Login failed or didn't return challenge_id"
    echo "   Response: $login_response"
    exit 1
end

echo "✅ Login successful, got challenge_id: $challenge_id"
echo ""

# Step 5: Test MFA verification with dummy code
echo "🎫 Step 5: Testing MFA verification with bypass code..."
set -l verify_response (curl -s -X POST "$API_BASE/v1/auth/verify-login" \
    -H "Content-Type: application/json" \
    -d "{\"challenge_id\":\"$challenge_id\",\"code\":\"000000\"}")

set -l token (echo $verify_response | jq -r '.token')
if test "$token" = "null" -o -z "$token"
    echo "❌ ERROR: MFA verification failed"
    echo "   Response: $verify_response"
    echo ""
    echo "🔍 Troubleshooting:"
    echo "   1. Check if MFA_BYPASS=true in docker-compose"
    echo "   2. Restart services: docker-compose restart api-gateway"
    echo "   3. Check logs: docker logs brokerx-api-gateway"
    exit 1
end

echo "✅ MFA verification successful with bypass code!"
echo "   Token: $token[1..50]..."
echo ""

# Step 6: Test authenticated endpoint
echo "🎯 Step 6: Testing authenticated endpoint..."
set -l portfolio_response (curl -s -X GET "$API_BASE/v1/portfolio" \
    -H "Authorization: Bearer $token")

set -l portfolio_check (echo $portfolio_response | jq -r '.balance // .error // "unknown"')
if test "$portfolio_check" = "unknown"
    echo "⚠️  WARNING: Unexpected portfolio response"
    echo "   Response: $portfolio_response"
else
    echo "✅ Authenticated request successful"
    echo "   Portfolio: $portfolio_check"
end
echo ""

# Final summary
echo "=================================="
echo "✅ MFA BYPASS TEST PASSED!"
echo "=================================="
echo ""
echo "Your load testing environment is ready!"
echo ""
echo "Next steps:"
echo "  1. Run smoke test: k6 run scenarios/market-data-consultation.js"
echo "  2. Run full test: k6 run --config config.js scenarios/complete-user-journey.js"
echo "  3. View metrics: http://localhost:3000 (Grafana)"
echo ""
