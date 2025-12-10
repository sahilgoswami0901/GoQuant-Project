"""
Collateral Vault Dashboard
Real-time monitoring dashboard for the Collateral Vault system
"""

import streamlit as st
import requests
import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
from datetime import datetime, timedelta
import time
from websocket_integration import create_websocket_component

# Configuration
BACKEND_URL = "http://localhost:8080"  # Change if backend runs on different port

# Page config
st.set_page_config(
    page_title="Collateral Vault Dashboard",
    page_icon="🏦",
    layout="wide",
    initial_sidebar_state="expanded"
)

# Custom CSS
st.markdown("""
<style>
    .metric-card {
        background-color: #0e1117;
        padding: 20px;
        border-radius: 10px;
        border: 1px solid #262730;
    }
    .stMetric {
        background-color: #0e1117;
    }
</style>
""", unsafe_allow_html=True)

def check_backend_health():
    """Check if backend is running"""
    try:
        response = requests.get(f"{BACKEND_URL}/health", timeout=2)
        return response.status_code == 200
    except:
        return False

def get_vault_balance(user_pubkey):
    """Get vault balance for a user"""
    try:
        response = requests.get(f"{BACKEND_URL}/vault/balance/{user_pubkey}")
        if response.status_code == 200:
            data = response.json()
            # Extract data from API response structure
            # API returns: {"success": true, "data": {...}}
            if isinstance(data, dict) and "data" in data:
                return data["data"]
            return data
        return None
    except Exception as e:
        st.error(f"Error fetching balance: {e}")
        return None

def get_transactions(user_pubkey):
    """Get transaction history for a user"""
    try:
        response = requests.get(f"{BACKEND_URL}/vault/transactions/{user_pubkey}")
        if response.status_code == 200:
            data = response.json()
            # Extract transactions from API response structure
            # API returns: {"success": true, "data": {"transactions": [...], ...}}
            if isinstance(data, dict) and "data" in data:
                return data["data"]
            return data
        return None
    except Exception as e:
        st.error(f"Error fetching transactions: {e}")
        return None

def get_tvl():
    """Get Total Value Locked"""
    try:
        # Use shorter timeout and handle errors gracefully
        response = requests.get(f"{BACKEND_URL}/vault/tvl", timeout=3)
        if response.status_code == 200:
            data = response.json()
            # Extract data from API response structure
            # API returns: {"success": true, "data": {...}}
            if isinstance(data, dict) and "data" in data:
                return data["data"]
            return data
        return None
    except requests.exceptions.ConnectionError:
        # Connection error - backend might be down or endpoint not available
        return None
    except requests.exceptions.Timeout:
        # Request timed out - TVL query might be slow
        return None
    except requests.exceptions.RequestException:
        # Any other request error
        return None
    except Exception:
        # Any other error - don't crash the dashboard
        return None

def format_usdt(amount):
    """Format USDT amount (6 decimals)"""
    return f"{amount / 1_000_000:,.2f} USDT"

# Sidebar
with st.sidebar:
    st.title("🏦 Collateral Vault")
    st.markdown("---")
    
    # Backend status
    if check_backend_health():
        st.success("✅ Backend Connected")
    else:
        st.error("❌ Backend Offline")
        st.info("Make sure backend is running on port 8080")
    
    st.markdown("---")
    
    # User input
    st.subheader("User Wallet")
    
    # Use session state to persist wallet address
    if "user_wallet" not in st.session_state:
        st.session_state.user_wallet = ""
    
    # Use a form for better UX with Enter key support
    with st.form("wallet_form", clear_on_submit=False):
        user_input = st.text_input(
            "Enter Solana Wallet Address",
            value=st.session_state.user_wallet,
            placeholder="GXvyLzGdAaKqs7ZPgxD7ALT1Rveto4T2YbJTo3RCGv7M",
            help="The Solana wallet address (public key) to view vault details",
            label_visibility="visible"
        )
        
        col1, col2 = st.columns(2)
        with col1:
            submitted = st.form_submit_button("🔍 View Vault", use_container_width=True)
        with col2:
            clear_btn = st.form_submit_button("🗑️ Clear", use_container_width=True)
        
        if submitted and user_input:
            st.session_state.user_wallet = user_input
            st.rerun()
        
        if clear_btn:
            st.session_state.user_wallet = ""
            st.rerun()
    
    # Update session state if input changed outside form
    if user_input and user_input != st.session_state.user_wallet:
        st.session_state.user_wallet = user_input
    
    st.markdown("---")
    
    # Real-time updates option
    st.subheader("Update Method")
    use_websocket = st.checkbox("🔴 Use WebSocket (Real-time)", value=True, 
                                help="Receive instant updates via WebSocket instead of polling")
    auto_refresh = st.checkbox("🔄 Auto-refresh (5s)", value=False,
                               help="Fallback polling method if WebSocket is disabled")
    
    # Manual refresh button
    if st.button("🔄 Refresh Balance", help="Manually refresh the balance from the backend"):
        st.rerun()
    
    if auto_refresh and not use_websocket:
        time.sleep(5)
        st.rerun()
    
    # Use session state value
    user_pubkey = st.session_state.user_wallet

# Main content
st.title("🏦 Collateral Vault Dashboard")
st.markdown("Real-time monitoring of vault balances and transactions")

# Check backend
if not check_backend_health():
    st.error("⚠️ Backend is not running!")
    st.info("""
    To start the backend:
    ```bash
    cd backend
    cargo run
    ```
    """)
    st.stop()

# TVL Overview
st.header("📊 Total Value Locked (TVL)")

# Try to get TVL data, but don't crash if it fails
try:
    tvl_data = get_tvl()
except Exception as e:
    # Silently fail - TVL is optional
    tvl_data = None

if tvl_data:
    # Helper function to get value with fallback for both naming conventions
    def get_tvl_value(key_snake, key_camel):
        value = tvl_data.get(key_camel) or tvl_data.get(key_snake) or 0
        if isinstance(value, str):
            try:
                return int(float(value))
            except:
                return 0
        return int(value) if value else 0
    
    # API returns camelCase: totalValueLocked, activeVaults, totalLocked, totalAvailable
    total_tvl = get_tvl_value("total_value_locked", "totalValueLocked")
    total_vaults = get_tvl_value("active_vaults", "activeVaults")
    total_locked = get_tvl_value("total_locked", "totalLocked")
    total_available = get_tvl_value("total_available", "totalAvailable")
    
    col1, col2, col3, col4 = st.columns(4)
    
    with col1:
        st.metric(
            "Total TVL",
            format_usdt(total_tvl),
            delta=None
        )
    
    with col2:
        st.metric(
            "Active Vaults",
            total_vaults,
            delta=None
        )
    
    with col3:
        st.metric(
            "Total Locked",
            format_usdt(total_locked),
            delta=None
        )
    
    with col4:
        st.metric(
            "Total Available",
            format_usdt(total_available),
            delta=None
        )
else:
    # Don't show error, just show info message
    st.info("📊 TVL data will appear here once vaults are created. This is optional and won't affect other features.")

st.markdown("---")

# User Vault Details
if user_pubkey:
    st.header(f"👤 User Vault: `{user_pubkey[:8]}...`")
    
    # WebSocket connection status (if enabled)
    if use_websocket:
        st.subheader("🔴 Real-time Updates")
        # Convert http:// to ws:// for WebSocket URL
        ws_url = BACKEND_URL.replace("http://", "ws://").replace("https://", "wss://")
        create_websocket_component(user_pubkey, ws_url)
        st.markdown("---")
    
    balance_data = get_vault_balance(user_pubkey)
    
    if balance_data:
        # Helper function to get value with fallback for both naming conventions
        def get_balance_value(key_snake, key_camel):
            value = balance_data.get(key_camel) or balance_data.get(key_snake) or 0
            # Convert to int if it's a string or float
            if isinstance(value, str):
                try:
                    return int(float(value))
                except:
                    return 0
            return int(value) if value else 0
        
        # Debug: Show raw balance data structure
        if st.checkbox("🔍 Debug: Show Balance Data", key="debug_balance"):
            st.write("Balance data structure:", balance_data)
            st.write("Available keys:", list(balance_data.keys()) if isinstance(balance_data, dict) else "Not a dict")
        
        # Extract balance values (handle both camelCase and snake_case)
        total_balance = get_balance_value("total_balance", "totalBalance")
        available_balance = get_balance_value("available_balance", "availableBalance")
        locked_balance = get_balance_value("locked_balance", "lockedBalance")
        
        # Balance metrics
        col1, col2, col3 = st.columns(3)
        
        with col1:
            st.metric(
                "Total Balance",
                format_usdt(total_balance),
                delta=None
            )
        
        with col2:
            st.metric(
                "Available Balance",
                format_usdt(available_balance),
                delta=None
            )
        
        with col3:
            st.metric(
                "Locked Balance",
                format_usdt(locked_balance),
                delta=None
            )
        
        # Balance breakdown chart
        st.subheader("Balance Breakdown")
        fig = go.Figure(data=[
            go.Bar(
                name="Available",
                x=["Balance"],
                y=[available_balance / 1_000_000],
                marker_color="#00cc96"
            ),
            go.Bar(
                name="Locked",
                x=["Balance"],
                y=[locked_balance / 1_000_000],
                marker_color="#ff6692"
            )
        ])
        fig.update_layout(
            barmode='stack',
            title="Vault Balance Distribution",
            yaxis_title="USDT",
            height=300
        )
        st.plotly_chart(fig, use_container_width=True)
        
        # Transaction history
        st.subheader("📜 Transaction History")
        transactions_data = get_transactions(user_pubkey)
        
        # Extract transactions array from API response
        transactions_list = []
        if transactions_data:
            if isinstance(transactions_data, dict):
                # API returns: {"transactions": [...], "total": 42, ...}
                if "transactions" in transactions_data:
                    transactions_list = transactions_data["transactions"]
                elif isinstance(transactions_data.get("data"), list):
                    transactions_list = transactions_data["data"]
            elif isinstance(transactions_data, list):
                transactions_list = transactions_data
        
        if transactions_list and len(transactions_list) > 0:
            # Convert to DataFrame
            df = pd.DataFrame(transactions_list)
            
            # Debug option (can be removed later)
            if st.checkbox("🔍 Debug: Show DataFrame Info", key="debug_transactions"):
                st.write("Available columns:", df.columns.tolist())
                st.write("DataFrame shape:", df.shape)
                st.write("Sample data:", df.head(3).to_dict('records'))
            
            # Format columns if they exist
            if "amount" in df.columns:
                df["amount_formatted"] = df["amount"].apply(
                    lambda x: format_usdt(x) if pd.notna(x) and x is not None else "0.00 USDT"
                )
            elif "formatted_amount" in df.columns:
                df["amount_formatted"] = df["formatted_amount"]
            else:
                df["amount_formatted"] = "N/A"
            
            # Handle date column (try different possible names)
            date_col = None
            for col in ["created_at", "createdAt", "timestamp", "date"]:
                if col in df.columns:
                    date_col = col
                    try:
                        df[col] = pd.to_datetime(df[col])
                    except:
                        pass
                    break
            
            # Build display columns - check what actually exists
            display_cols = []
            if "transaction_type" in df.columns or "transactionType" in df.columns:
                display_cols.append("transaction_type" if "transaction_type" in df.columns else "transactionType")
            if "amount_formatted" in df.columns:
                display_cols.append("amount_formatted")
            if "status" in df.columns:
                display_cols.append("status")
            if date_col:
                display_cols.append(date_col)
            
            # Display table with available columns
            if display_cols:
                st.dataframe(
                    df[display_cols].head(20),
                    use_container_width=True,
                    hide_index=True
                )
            else:
                # Fallback: show all available columns
                st.dataframe(
                    df.head(20),
                    use_container_width=True,
                    hide_index=True
                )
        
        else:
            st.info("No transactions found for this user")
        
        # Vault details
        with st.expander("📋 Vault Details"):
            st.json(balance_data)
    else:
        st.warning(f"Vault not found for user: {user_pubkey}")
        st.info("Make sure the vault has been initialized on-chain")
else:
    st.info("👈 Enter a user public key in the sidebar to view their vault")

# Footer
st.markdown("---")
st.markdown("""
<div style='text-align: center; color: #666;'>
    <p>Collateral Vault Dashboard | Built with Streamlit</p>
    <p>Backend API: <code>{}</code></p>
</div>
""".format(BACKEND_URL), unsafe_allow_html=True)
