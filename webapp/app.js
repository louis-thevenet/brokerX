// BrokerX Trading Platform - Professional Web Application
const API_BASE_URL = 'http://localhost:8080';
let currentToken = localStorage.getItem('brokerx_token');
let currentUser = null;

// MFA state management (mandatory for all users)
let pendingMfaChallenge = null;
let mfaStep = null; // 'registration' or 'login'

// Initialize app on load
document.addEventListener('DOMContentLoaded', function() {
  if (currentToken) {
    loadUserProfile();
    showDashboard();
  } else {
    showAuthSection();
  }
  setupMfaAutoSubmit();
});

// Utility Functions
function showMessage(elementId, message, isError = false) {
  const element = document.getElementById(elementId);
  element.innerHTML = `<p class="${isError ? 'error' : 'success'}">${message}</p>`;
  setTimeout(() => element.innerHTML = '', 5000);
}

function makeApiCall(endpoint, options = {}) {
  const defaultOptions = {
    headers: {
      'Content-Type': 'application/json',
    }
  };

  if (currentToken) {
    defaultOptions.headers['Authorization'] = `Bearer ${currentToken}`;
  }

  const finalOptions = {
    ...defaultOptions,
    ...options,
    headers: { ...defaultOptions.headers, ...options.headers }
  };

  return fetch(`${API_BASE_URL}${endpoint}`, finalOptions);
}

// Authentication Functions
async function login(event) {
  event.preventDefault();

  const email = document.getElementById('login-email').value;
  const password = document.getElementById('login-password').value;

  try {
    const response = await makeApiCall('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password })
    });

    if (response.ok) {
      const data = await response.json();
      
      // All logins require MFA - always show verification screen
      pendingMfaChallenge = data.challenge_id;
      mfaStep = 'login';
      sessionStorage.setItem('pending_user_id', data.user_id);
      showMfaVerification('Multi-Factor Authentication', 
        'Please enter the verification code sent to your email to complete login. Check your email for the 6-digit code.');
    } else {
      const error = await response.json();
      showMessage('auth-message', error.message || 'Login failed', true);
    }
  } catch (err) {
    showMessage('auth-message', 'Network error occurred', true);
    console.error('Login error:', err);
  }
}

async function register(event) {
  event.preventDefault();

  const email = document.getElementById('reg-email').value;
  const password = document.getElementById('reg-password').value;
  const firstname = document.getElementById('reg-firstname').value;
  const surname = document.getElementById('reg-surname').value;

  try {
    const response = await makeApiCall('/auth/register', {
      method: 'POST',
      body: JSON.stringify({
        email,
        password,
        firstname,
        surname,
      })
    });

    if (response.ok) {
      const data = await response.json();
      
      // Registration creates the account - MFA challenge will be sent during login
      showMessage('auth-message', 
        'Registration successful! Please login to verify your account. You will receive a verification code via email during login.');
      
      // Clear registration form and show login
      document.getElementById('reg-email').value = '';
      document.getElementById('reg-password').value = '';
      document.getElementById('reg-firstname').value = '';
      document.getElementById('reg-surname').value = '';
      showLogin();
    } else {
      const error = await response.json();
      showMessage('auth-message', error.message || 'Registration failed', true);
    }
  } catch (err) {
    showMessage('auth-message', 'Network error occurred', true);
    console.error('Registration error:', err);
  }
}

function shouldRequireMfa(email) {
  // MFA is mandatory for all users in this professional trading application
  // This ensures compliance with financial security regulations
  return true;
}

function logout() {
  currentToken = null;
  currentUser = null;
  localStorage.removeItem('brokerx_token');
  showAuthSection();
  showMessage('auth-message', 'Logged out successfully');
}

// User Profile Functions
async function loadUserProfile() {
  try {
    const response = await makeApiCall('/me');

    if (response.ok) {
      currentUser = await response.json();
      updateUserInfo();
    } else if (response.status === 401) {
      // Token expired
      logout();
    } else {
      showMessage('profile-message', 'Failed to load profile', true);
    }
  } catch (err) {
    showMessage('profile-message', 'Network error occurred', true);
    console.error('Profile load error:', err);
  }
}

function updateUserInfo() {
  if (!currentUser) return;

  document.getElementById('user-info').innerHTML = `
        <h3>Welcome, ${currentUser.firstname} ${currentUser.surname}!</h3>
        <p><strong>Email:</strong> ${currentUser.email} ${currentUser.is_verified ? '✅' : '❌ (Not Verified)'}</p>
        <p><strong>Balance:</strong> $${currentUser.balance.toFixed(2)}</p>
        <p><strong>Member since:</strong> ${new Date(currentUser.created_at).toLocaleDateString()}</p>
    `;
}

async function updateProfile(event) {
  event.preventDefault();

  const email = document.getElementById('update-email').value;
  const firstname = document.getElementById('update-firstname').value;
  const surname = document.getElementById('update-surname').value;

  const updateData = {};
  if (email) updateData.email = email;
  if (firstname) updateData.firstname = firstname;
  if (surname) updateData.surname = surname;

  if (Object.keys(updateData).length === 0) {
    showMessage('profile-message', 'Please fill at least one field to update', true);
    return;
  }

  try {
    const response = await makeApiCall('/me', {
      method: 'PUT',
      body: JSON.stringify(updateData)
    });

    if (response.ok) {
      showMessage('profile-message', 'Profile updated successfully!');
      await loadUserProfile();
      // Clear form
      document.getElementById('update-email').value = '';
      document.getElementById('update-firstname').value = '';
      document.getElementById('update-surname').value = '';
    } else {
      const error = await response.json();
      showMessage('profile-message', error.message || 'Update failed', true);
    }
  } catch (err) {
    showMessage('profile-message', 'Network error occurred', true);
    console.error('Update error:', err);
  }
}

async function verifyEmail(event) {
  event.preventDefault();

  // In the new flow, email verification happens during login
  // If user is already logged in but not verified, they need to logout and login again
  showMessage('profile-message', 
    'Email verification must be completed during login. Please logout and login again to verify your email.', 
    true);
}

async function depositFunds(event) {
  event.preventDefault();

  const amount = parseFloat(document.getElementById('deposit-amount').value);

  try {
    const response = await makeApiCall('/me/deposit', {
      method: 'POST',
      body: JSON.stringify({ amount })
    });

    if (response.ok) {
      showMessage('profile-message', `Successfully deposited $${amount.toFixed(2)}!`);
      await loadUserProfile();
      document.getElementById('deposit-amount').value = '';
    } else {
      const error = await response.json();
      showMessage('profile-message', error.message || 'Deposit failed', true);
    }
  } catch (err) {
    showMessage('profile-message', 'Network error occurred', true);
    console.error('Deposit error:', err);
  }
}

// Trading Functions
async function placeOrder(event) {
  event.preventDefault();

  const symbol = document.getElementById('order-symbol').value.toUpperCase();
  const quantity = parseInt(document.getElementById('order-quantity').value);
  const order_side = document.getElementById('order-side').value;
  const order_type = document.getElementById('order-type').value;
  const price = document.getElementById('order-price').value;

  const orderData = {
    symbol,
    quantity,
    order_side,
    order_type
  };

  if (order_type === 'LIMIT' && price) {
    orderData.price = parseFloat(price);
  }

  try {
    const response = await makeApiCall('/orders', {
      method: 'POST',
      body: JSON.stringify(orderData)
    });

    if (response.ok) {
      const order = await response.json();
      showMessage('trading-message', `Order placed successfully! Order ID: ${order.id}`);
      // Clear form
      document.getElementById('order-symbol').value = '';
      document.getElementById('order-quantity').value = '';
      document.getElementById('order-side').value = '';
      document.getElementById('order-type').value = '';
      document.getElementById('order-price').value = '';
      document.getElementById('order-price').classList.add('hidden');

      // Refresh orders and profile
      await loadUserProfile();
      if (!document.getElementById('orders-section').classList.contains('hidden')) {
        await loadOrders();
      }
    } else {
      const error = await response.json();
      showMessage('trading-message', error.message || 'Order placement failed', true);
    }
  } catch (err) {
    showMessage('trading-message', 'Network error occurred', true);
    console.error('Order placement error:', err);
  }
}

function togglePriceField() {
  const orderType = document.getElementById('order-type').value;
  const priceField = document.getElementById('order-price');

  if (orderType === 'LIMIT') {
    priceField.classList.remove('hidden');
    priceField.required = true;
  } else {
    priceField.classList.add('hidden');
    priceField.required = false;
    priceField.value = '';
  }
}

// Orders Functions
async function loadOrders() {
  try {
    const response = await makeApiCall('/orders');

    if (response.ok) {
      const data = await response.json();
      displayOrders(data.data || data);
    } else {
      const error = await response.json();
      showMessage('orders-message', error.message || 'Failed to load orders', true);
    }
  } catch (err) {
    showMessage('orders-message', 'Network error occurred', true);
    console.error('Orders load error:', err);
  }
}

function displayOrders(orders) {
  const tableContainer = document.getElementById('orders-table');

  if (!orders || orders.length === 0) {
    tableContainer.innerHTML = '<p>No orders found.</p>';
    return;
  }

  let html = `
    <table>
      <thead>
        <tr>
          <th>Symbol</th>
          <th>Side</th>
          <th>Type</th>
          <th>Quantity</th>
          <th>Price</th>
          <th>Status</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
  `;

  orders.forEach(order => {
    html += `
      <tr>
        <td>${order.symbol}</td>
        <td>${order.order_side}</td>
        <td>${order.order_type}</td>
        <td>${order.quantity}</td>
        <td>${order.price ? '$' + order.price.toFixed(2) : 'Market'}</td>
        <td>${order.status}</td>
        <td>
          ${order.status === 'PENDING' ? 
            `<button onclick="cancelOrder('${order.id}')" class="btn-secondary">Cancel</button>` : 
            '-'
          }
        </td>
      </tr>
    `;
  });

  html += '</tbody></table>';
  tableContainer.innerHTML = html;
}



async function cancelOrder(orderId) {
  if (!confirm('Are you sure you want to cancel this order?')) {
    return;
  }

  try {
    const response = await makeApiCall(`/orders/${orderId}`, {
      method: 'DELETE'
    });

    if (response.ok) {
      showMessage('orders-message', 'Order cancelled successfully!');
      await loadOrders();
      await loadUserProfile();
    } else {
      const error = await response.json();
      showMessage('orders-message', error.message || 'Failed to cancel order', true);
    }
  } catch (err) {
    showMessage('orders-message', 'Network error occurred', true);
    console.error('Cancel order error:', err);
  }
}

// MFA Functions
async function verifyMfa(event) {
  event.preventDefault();

  if (!pendingMfaChallenge) {
    showMessage('mfa-message', 'No active MFA challenge. Please login again.', true);
    clearMfaState();
    showAuthSection();
    return;
  }

  const code = document.getElementById('mfa-code').value;

  if (!code || code.length !== 6) {
    showMessage('mfa-message', 'Please enter a valid 6-digit code', true);
    return;
  }

  try {
    // Both registration and login verification use the same endpoint
    const response = await makeApiCall('/auth/verify-login', {
      method: 'POST',
      body: JSON.stringify({
        challenge_id: pendingMfaChallenge,
        code: code
      })
    });

    if (response.ok) {
      const data = await response.json();
      
      // Save token and login
      currentToken = data.token;
      localStorage.setItem('brokerx_token', currentToken);
      sessionStorage.removeItem('pending_user_id');
      
      showMessage('auth-message', 'Verification successful! You are now logged in.');
      
      clearMfaState();
      await loadUserProfile();
      showDashboard();
    } else {
      const error = await response.json();
      showMessage('mfa-message', error.message || 'Verification failed', true);
    }
  } catch (err) {
    showMessage('mfa-message', 'Network error occurred', true);
    console.error('MFA verification error:', err);
  }
}

async function resendMfaCode() {
  if (!pendingMfaChallenge) {
    showMessage('mfa-message', 'No active MFA challenge', true);
    return;
  }

  // Simulate resending MFA code
  showMessage('mfa-message', 'Code resent! Demo code: 123456');
}

function showMfaVerification(title, description) {
  // Hide other sections
  document.getElementById('auth-section').classList.add('hidden');
  document.getElementById('dashboard-section').classList.add('hidden');
  document.getElementById('profile-section').classList.add('hidden');
  document.getElementById('trading-section').classList.add('hidden');
  document.getElementById('orders-section').classList.add('hidden');
  
  // Show MFA section
  document.getElementById('mfa-section').classList.remove('hidden');
  document.getElementById('mfa-title').textContent = title;
  document.getElementById('mfa-description').textContent = description;
  document.getElementById('mfa-code').value = '';
  document.getElementById('mfa-code').focus();
}

function clearMfaState() {
  pendingMfaChallenge = null;
  mfaStep = null;
  document.getElementById('mfa-section').classList.add('hidden');
}

function cancelMfa() {
  clearMfaState();
  showAuthSection();
}

// UI Navigation Functions
function showAuthSection() {
  document.getElementById('auth-section').classList.remove('hidden');
  document.getElementById('mfa-section').classList.add('hidden');
  document.getElementById('dashboard-section').classList.add('hidden');
  document.getElementById('profile-section').classList.add('hidden');
  document.getElementById('trading-section').classList.add('hidden');
  document.getElementById('orders-section').classList.add('hidden');
  clearMfaState();
}

function showDashboard() {
  document.getElementById('auth-section').classList.add('hidden');
  document.getElementById('mfa-section').classList.add('hidden');
  document.getElementById('dashboard-section').classList.remove('hidden');
  document.getElementById('profile-section').classList.remove('hidden');
  document.getElementById('trading-section').classList.remove('hidden');
  document.getElementById('orders-section').classList.remove('hidden');

  // Load orders when showing dashboard
  loadOrders();
}

function showLogin() {
  document.getElementById('login-form').classList.remove('hidden');
  document.getElementById('register-form').classList.add('hidden');
}

function showRegister() {
  document.getElementById('login-form').classList.add('hidden');
  document.getElementById('register-form').classList.remove('hidden');
}

// Health check function (for development)
async function checkApiHealth() {
  try {
    const response = await fetch(`${API_BASE_URL}/health`);
    if (response.ok) {
      const data = await response.json();
      console.log('API Health:', data);
      return true;
    }
  } catch (err) {
    console.error('API Health check failed:', err);
  }
  return false;
}

// Check API health on load
checkApiHealth();

// Auto-submit MFA code when 6 digits are entered
document.addEventListener('DOMContentLoaded', function() {
  // This will be set up after the DOM is loaded
  setupMfaAutoSubmit();
});

function setupMfaAutoSubmit() {
  const mfaCodeInput = document.getElementById('mfa-code');
  if (mfaCodeInput) {
    mfaCodeInput.addEventListener('input', function(e) {
      // Only allow numeric input
      e.target.value = e.target.value.replace(/[^0-9]/g, '');
      
      // Auto-submit when 6 digits are entered
      if (e.target.value.length === 6) {
        // Small delay to allow user to see the complete code
        setTimeout(() => {
          const form = e.target.closest('form');
          if (form) {
            form.dispatchEvent(new Event('submit'));
          }
        }, 500);
      }
    });

    // Focus the input when MFA section becomes visible
    const observer = new MutationObserver(function(mutations) {
      mutations.forEach(function(mutation) {
        if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
          const mfaSection = document.getElementById('mfa-section');
          if (mfaSection && !mfaSection.classList.contains('hidden')) {
            setTimeout(() => mfaCodeInput.focus(), 100);
          }
        }
      });
    });

    const mfaSection = document.getElementById('mfa-section');
    if (mfaSection) {
      observer.observe(mfaSection, { attributes: true });
    }
  }
}
