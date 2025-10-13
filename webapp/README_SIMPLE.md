# BrokerX Trading Platform - Web Application

A professional web application for secure trading operations with mandatory multi-factor authentication.

## 🚀 Core Features

- **Secure Authentication**: Login/registration with mandatory MFA
- **Profile Management**: View balance, update information, deposit funds  
- **Trading Operations**: Place market/limit orders for stocks
- **Order Management**: View and cancel pending orders
- **Security Compliance**: MFA required for all users

## 🛠️ Quick Start

1. **Start Backend Services**
   ```bash
   # From workspace root
   ./start-all.sh
   ```

2. **Open Web Application**
   ```bash
   cd webapp
   python3 -m http.server 3000
   # Visit http://localhost:3000
   ```

## 📖 User Guide

### Registration & Login
1. **Register**: Fill form → Complete MFA verification (demo code: 123456)
2. **Login**: Enter credentials → Complete MFA verification (demo code: 123456)
3. **Access Granted**: View dashboard with account information

### Core Operations
- **Deposit Funds**: Add money to your trading account
- **Place Orders**: Buy/sell stocks (Market or Limit orders)
- **View Orders**: Monitor all your trading activity
- **Cancel Orders**: Cancel pending orders before execution

### Security
- **MFA Required**: All users must complete multi-factor authentication
- **Demo Mode**: Use verification code "123456" for demonstration
- **Secure Sessions**: JWT tokens protect all trading operations

## 🎯 User Cases Covered

1. **Account Creation**: Register → MFA verification → Login access
2. **Secure Login**: Credentials → MFA verification → Dashboard access  
3. **Fund Management**: Deposit funds → Updated balance → Ready to trade
4. **Order Placement**: Enter order details → Submit → Order tracking
5. **Order Management**: View orders → Cancel if needed → Portfolio management

## ⚡ Technical Details

- **Frontend**: Vanilla HTML/CSS/JavaScript
- **Authentication**: JWT with mandatory MFA
- **API**: RESTful communication with backend services
- **Security**: All endpoints require authentication tokens

This webapp demonstrates a complete, secure trading platform with professional-grade security features.
