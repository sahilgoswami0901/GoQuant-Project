# 🚀 Backend Setup & Running Guide

Complete guide to set up and run the Collateral Vault backend with real-time monitoring.

## 📋 Prerequisites

1. **PostgreSQL** (via Docker)
2. **Rust & Cargo** installed
3. **Solana CLI** installed
4. **Program deployed** to devnet/localnet

---

## 🗄️ Step 1: Setup Database

```bash
# Start PostgreSQL container
docker run -d \
  --name collateral-vault-db \
  -p 5432:5432 \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=collateral_vault \
  -v collateral_vault_data:/var/lib/postgresql/data \
  postgres:15

# Wait for it to start
sleep 5

# Run migrations
docker exec -i collateral-vault-db psql -U postgres -d collateral_vault < backend/migrations/001_initial_schema.sql
```

---

## ⚙️ Step 2: Configure Environment

Make sure your `backend/.env` file is set up:

```env
# Database
DATABASE_URL=postgres://postgres:password@localhost:5432/collateral_vault

# Solana (Devnet)
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_WS_URL=wss://api.devnet.solana.com
VAULT_PROGRAM_ID=YOUR_PROGRAM_ID_HERE

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
```

**Important:** Replace `YOUR_PROGRAM_ID_HERE` with your deployed program ID!

---

## 🏃 Step 3: Run the Backend

```bash
cd backend
cargo run
```

You should see:
```
🚀 Starting Collateral Vault Backend Service
📋 Configuration loaded
🗄️  Database connected
⛓️  Solana client initialized
🌐 Starting HTTP server on 127.0.0.1:8080
```

---

## 🧪 Step 4: Test the API

### Health Check
```bash
curl http://localhost:8080/health
```

### Get Vault Balance
```bash
curl http://localhost:8080/vault/balance/YOUR_USER_PUBKEY
```

### Get TVL
```bash
curl http://localhost:8080/vault/tvl
```

---

## 📊 Step 5: Run the Dashboard

### Install Dashboard Dependencies
```bash
cd dashboard
pip install -r requirements.txt
```

### Start Streamlit Dashboard
```bash
streamlit run app.py
```

Dashboard will open at: **http://localhost:8501**

---

## 🔍 Available API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/vault/initialize` | POST | Initialize new vault |
| `/vault/deposit` | POST | Deposit USDT |
| `/vault/withdraw` | POST | Withdraw USDT |
| `/vault/balance/{user}` | GET | Get vault balance |
| `/vault/transactions/{user}` | GET | Get transaction history |
| `/vault/tvl` | GET | Total Value Locked |

---

## 🐛 Troubleshooting

### Backend won't start

**Error: Database connection failed**
```bash
# Check if PostgreSQL is running
docker ps

# Check logs
docker logs collateral-vault-db
```

**Error: Solana RPC connection failed**
- Verify `SOLANA_RPC_URL` in `.env`
- Check your internet connection
- For devnet, ensure you have valid RPC endpoint

**Error: Program ID not found**
- Make sure `VAULT_PROGRAM_ID` in `.env` matches your deployed program
- Verify program is deployed: `solana program show YOUR_PROGRAM_ID`

### Dashboard shows "Backend Offline"

1. Check backend is running: `curl http://localhost:8080/health`
2. Verify `BACKEND_URL` in `dashboard/app.py` matches your backend port
3. Check backend logs for errors

---

## 📈 Real-time Monitoring

The backend includes:

- **Vault Monitor** - Tracks on-chain vault changes
- **Balance Tracker** - Reconciles on-chain vs off-chain state
- **WebSocket Support** - Real-time updates (coming soon)

---

## 🔄 Background Services

The backend automatically runs:

1. **Vault Monitor** - Watches for new vaults and updates
2. **Balance Reconciliation** - Ensures database matches blockchain
3. **Alert System** - Monitors low balances

These run in the background - no action needed!

---

## 📝 Next Steps

1. ✅ Database setup
2. ✅ Backend running
3. ✅ Dashboard running
4. 🎯 Deploy to production
5. 🎯 Add authentication
6. 🎯 Set up monitoring/alerting

---

## 🆘 Need Help?

- Check backend logs: Look at terminal output
- Check database: `docker exec -it collateral-vault-db psql -U postgres -d collateral_vault`
- Check Solana: `solana cluster-version`
