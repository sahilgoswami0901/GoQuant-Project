# WebSocket Real-Time Updates Guide

## 🎯 Who Uses WebSockets and Why?

### **Primary Users: Frontend Applications (Web/Mobile Apps)**

WebSockets are used by **frontend applications** (React, Vue, Angular, mobile apps) that need to display **real-time updates** to users without constantly polling the API.

---

## 📱 Real-World Use Cases

### **1. Trading Dashboard / DeFi Frontend**

**Who:** Users trading on your perpetual futures DEX

**What They See:**
- **Real-time balance updates** when they deposit/withdraw
- **Instant transaction confirmations** when their trades execute
- **Live collateral status** when positions are opened/closed
- **TVL (Total Value Locked) updates** for the entire platform

**Why WebSockets?**
- **No polling needed**: Instead of checking balance every 2-3 seconds (wasteful), updates arrive instantly
- **Better UX**: Users see changes immediately, not after a refresh
- **Lower server load**: One WebSocket connection vs. hundreds of HTTP requests per minute

### **2. Portfolio Tracker**

**Who:** Users monitoring their vault across multiple devices

**What They See:**
- Balance changes on their phone, tablet, and desktop **simultaneously**
- Notifications when transactions complete
- Real-time updates when collateral is locked/unlocked by trading positions

**Why WebSockets?**
- **Multi-device sync**: All devices get updates at the same time
- **Battery efficient**: Mobile apps don't need to constantly poll

### **3. Admin Dashboard**

**Who:** Platform administrators monitoring system health

**What They See:**
- Real-time TVL (Total Value Locked) across all vaults
- System health updates
- Transaction volume metrics

**Why WebSockets?**
- **Live monitoring**: See changes as they happen
- **Alert system**: Can trigger alerts based on WebSocket events

---

## 🔄 How It Works

### **Traditional Approach (Without WebSockets):**

```javascript
// ❌ BAD: Polling every 2 seconds
setInterval(async () => {
  const balance = await fetch('/vault/balance/USER_ADDRESS');
  updateUI(balance);
}, 2000); // Wastes bandwidth, server resources
```

**Problems:**
- ❌ Wastes bandwidth (checking even when nothing changed)
- ❌ High server load (thousands of requests per minute)
- ❌ Delayed updates (up to 2 seconds delay)
- ❌ Battery drain on mobile

### **WebSocket Approach (What We Built):**

```javascript
// ✅ GOOD: Real-time updates
const ws = new WebSocket('ws://localhost:8080/ws/USER_ADDRESS');

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  if (message.event === 'balance_update') {
    // Update UI immediately when balance changes
    updateBalance(message.data);
  }
  
  if (message.event === 'transaction_confirmed') {
    // Show notification when transaction completes
    showNotification('Deposit confirmed!');
  }
};
```

**Benefits:**
- ✅ **Instant updates**: Changes arrive immediately
- ✅ **Efficient**: Only sends data when something changes
- ✅ **Low latency**: No polling delay
- ✅ **Battery friendly**: Mobile apps don't constantly poll

---

## 📊 Event Types We Send

### **1. Balance Update** (`balance_update`)

**When:** After deposit, withdraw, lock, or unlock operations

**Who Gets It:** The user whose vault changed

**Example:**
```json
{
  "event": "balance_update",
  "data": {
    "owner": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "totalBalance": 1000000000,
    "lockedBalance": 200000000,
    "availableBalance": 800000000
  },
  "timestamp": "2025-12-08T21:30:00Z"
}
```

**Frontend Use:**
```javascript
// Update balance display in real-time
if (message.event === 'balance_update') {
  document.getElementById('total-balance').textContent = 
    formatUSDT(message.data.totalBalance);
  document.getElementById('available-balance').textContent = 
    formatUSDT(message.data.availableBalance);
}
```

---

### **2. Transaction Confirmed** (`transaction_confirmed`)

**When:** When a deposit or withdraw transaction is confirmed on-chain

**Who Gets It:** The user who initiated the transaction

**Example:**
```json
{
  "event": "transaction_confirmed",
  "data": {
    "transactionId": "a73028b9-6e47-4e8c-8ffd-5f13ecc797a7",
    "transactionType": "deposit",
    "amount": 100000000,
    "signature": "5j7Ks8...abc123"
  },
  "timestamp": "2025-12-08T21:30:05Z"
}
```

**Frontend Use:**
```javascript
// Show success notification
if (message.event === 'transaction_confirmed') {
  showToast(`✅ ${message.data.transactionType} confirmed!`);
  // Update transaction history
  addToTransactionHistory(message.data);
}
```

---

### **3. Collateral Locked** (`collateral_locked`)

**When:** When a trading position locks collateral

**Who Gets It:** The vault owner

**Example:**
```json
{
  "event": "collateral_locked",
  "data": {
    "owner": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "amount": 50000000,
    "positionId": "pos_12345",
    "lockedBalance": 250000000,
    "availableBalance": 750000000
  },
  "timestamp": "2025-12-08T21:31:00Z"
}
```

**Frontend Use:**
```javascript
// Update position display
if (message.event === 'collateral_locked') {
  updatePositionStatus(message.data.positionId, 'locked');
  updateAvailableBalance(message.data.availableBalance);
}
```

---

### **4. Collateral Unlocked** (`collateral_unlocked`)

**When:** When a trading position closes and unlocks collateral

**Who Gets It:** The vault owner

**Example:**
```json
{
  "event": "collateral_unlocked",
  "data": {
    "owner": "HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m",
    "amount": 50000000,
    "positionId": "pos_12345",
    "lockedBalance": 200000000,
    "availableBalance": 800000000
  },
  "timestamp": "2025-12-08T21:32:00Z"
}
```

---

## 🚀 How to Connect (Frontend Example)

### **JavaScript/TypeScript:**

```javascript
// Connect to WebSocket
const userAddress = 'HjxQC5jmM8JQrKmVLz29fvaVU428BQ3NzM7a3uQECt9m';
const ws = new WebSocket(`ws://localhost:8080/ws/${userAddress}`);

// Handle connection open
ws.onopen = () => {
  console.log('✅ WebSocket connected');
};

// Handle incoming messages
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  switch (message.event) {
    case 'balance_update':
      updateBalanceDisplay(message.data);
      break;
      
    case 'transaction_confirmed':
      showTransactionNotification(message.data);
      break;
      
    case 'collateral_locked':
      updatePositionStatus(message.data);
      break;
      
    case 'collateral_unlocked':
      updatePositionStatus(message.data);
      break;
      
    case 'health_update':
      console.log('Connection status:', message.data.status);
      break;
  }
};

// Handle errors
ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

// Handle disconnection
ws.onclose = () => {
  console.log('WebSocket disconnected, reconnecting...');
  // Implement reconnection logic
  setTimeout(() => {
    // Reconnect
  }, 3000);
};
```

---

### **React Example:**

```tsx
import { useEffect, useState } from 'react';

function VaultDashboard({ userAddress }: { userAddress: string }) {
  const [balance, setBalance] = useState({ total: 0, available: 0, locked: 0 });
  const [notifications, setNotifications] = useState<string[]>([]);

  useEffect(() => {
    const ws = new WebSocket(`ws://localhost:8080/ws/${userAddress}`);

    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);

      if (message.event === 'balance_update') {
        setBalance({
          total: message.data.totalBalance,
          available: message.data.availableBalance,
          locked: message.data.lockedBalance,
        });
      }

      if (message.event === 'transaction_confirmed') {
        setNotifications(prev => [
          ...prev,
          `${message.data.transactionType} confirmed!`
        ]);
      }
    };

    return () => ws.close();
  }, [userAddress]);

  return (
    <div>
      <h2>Vault Balance</h2>
      <p>Total: {formatUSDT(balance.total)}</p>
      <p>Available: {formatUSDT(balance.available)}</p>
      <p>Locked: {formatUSDT(balance.locked)}</p>
      
      {notifications.map((notif, i) => (
        <div key={i}>{notif}</div>
      ))}
    </div>
  );
}
```

---

## 🎨 Real-World Example: Trading Interface

Imagine a user is trading on your platform:

1. **User opens trading interface** → WebSocket connects
2. **User deposits 100 USDT** → Frontend shows "Processing..."
3. **Transaction confirms on-chain** → WebSocket receives `transaction_confirmed`
4. **Frontend updates** → Balance shows 100 USDT instantly
5. **User opens a position** → Trading program locks 50 USDT
6. **WebSocket receives `collateral_locked`** → Frontend updates:
   - Available balance: 50 USDT
   - Locked balance: 50 USDT
   - Position status: "Active"
7. **User closes position** → WebSocket receives `collateral_unlocked`
8. **Frontend updates** → Available balance back to 100 USDT

**All of this happens in real-time without the user refreshing the page!**

---

## 🔧 Technical Details

### **Connection Endpoint:**
```
ws://localhost:8080/ws/{user_pubkey}
```

### **Message Format:**
All messages are JSON with this structure:
```json
{
  "event": "event_type",
  "data": { /* event-specific data */ },
  "timestamp": "ISO 8601 timestamp"
}
```

### **Connection Lifecycle:**
1. Client connects → Server sends welcome message
2. Client stays connected → Receives events as they occur
3. Client disconnects → Server cleans up connection
4. Client reconnects → Can reconnect anytime

### **Multiple Connections:**
- Same user can connect from multiple devices/tabs
- All connections receive the same events
- Each connection is independent

---

## 📈 Benefits Summary

| Feature | Without WebSockets | With WebSockets |
|---------|-------------------|-----------------|
| **Update Speed** | 2-3 second delay | Instant |
| **Server Load** | High (constant polling) | Low (event-driven) |
| **Bandwidth** | High (redundant requests) | Low (only when needed) |
| **Battery (Mobile)** | Drains quickly | Efficient |
| **User Experience** | "Stale" data | Real-time updates |

---

## 🎓 Summary

**WebSockets enable real-time, bidirectional communication between your backend and frontend applications.**

**Who uses them:**
- Frontend web applications (React, Vue, etc.)
- Mobile apps (React Native, Flutter, etc.)
- Trading dashboards
- Admin panels
- Any app needing live updates

**Why they're important:**
- **Better UX**: Users see changes instantly
- **Efficiency**: No wasteful polling
- **Scalability**: Lower server load
- **Modern**: Industry standard for real-time apps

Your collateral vault system now provides **real-time updates** to any frontend that connects, making it perfect for building a modern DeFi trading interface! 🚀

