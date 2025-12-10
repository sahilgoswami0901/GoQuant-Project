# Collateral Vault Dashboard

Real-time Streamlit dashboard for monitoring the Collateral Vault system.

## 🚀 Quick Start

### 1. Install Dependencies

```bash
cd dashboard
pip install -r requirements.txt
```

### 2. Start the Dashboard

```bash
streamlit run app.py
```

The dashboard will open at `http://localhost:8501`

## 📋 Prerequisites

- Backend must be running on `http://localhost:8080`
- PostgreSQL database must be set up
- Solana program deployed (devnet or localnet)

## 🎯 Features

- **Total Value Locked (TVL)** - Overview of all vaults
- **User Vault Details** - View individual vault balances
- **Transaction History** - See all deposits/withdrawals
- **Real-time Updates** - Auto-refresh option
- **Visual Charts** - Balance breakdown and transaction timeline

## 🔧 Configuration

Edit `BACKEND_URL` in `app.py` if your backend runs on a different port:

```python
BACKEND_URL = "http://localhost:8080"  # Change this
```
