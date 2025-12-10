# Database Diagram Prompt for dbdiagram.io

## Quick Start

1. Go to [https://dbdiagram.io/d](https://dbdiagram.io/d)
2. Click on the editor
3. Copy the entire content from `DB_DIAGRAM_PROMPT.txt`
4. Paste it into the editor
5. The diagram will be automatically generated!

## What's Included

The prompt includes:

- **6 Tables**: vaults, transactions, balance_snapshots, reconciliation_logs, tvl_snapshots, alerts
- **All Relationships**: Foreign keys and their connections
- **Field Descriptions**: Notes explaining each field
- **Indexes**: All database indexes for performance
- **Constraints**: Important business rules

## Relationships

```
vaults (1) ──< (many) transactions
vaults (1) ──< (many) balance_snapshots
vaults (1) ──< (many) reconciliation_logs (optional)
vaults (1) ──< (many) alerts (optional)
tvl_snapshots (standalone - no foreign keys)
```

## Table Descriptions

### vaults
- **Primary Key**: `owner` (user's wallet public key)
- **Purpose**: Cached vault data from blockchain
- **Key Fields**: Balance tracking (total, locked, available)

### transactions
- **Primary Key**: `id` (UUID)
- **Foreign Key**: `vault_owner` → `vaults.owner`
- **Purpose**: Complete transaction history for auditing
- **Key Fields**: Transaction type, amount, status, signatures

### balance_snapshots
- **Primary Key**: `id` (UUID)
- **Foreign Key**: `vault_owner` → `vaults.owner`
- **Purpose**: Periodic balance recordings for analytics
- **Key Fields**: Timestamp, snapshot type

### reconciliation_logs
- **Primary Key**: `id` (UUID)
- **Foreign Key**: `vault_owner` → `vaults.owner` (optional)
- **Purpose**: Audit trail for balance reconciliation
- **Key Fields**: Expected vs actual balance, difference

### tvl_snapshots
- **Primary Key**: `id` (UUID)
- **Purpose**: System-wide Total Value Locked metrics
- **Key Fields**: Total TVL, active vaults count

### alerts
- **Primary Key**: `id` (UUID)
- **Foreign Key**: `vault_owner` → `vaults.owner` (optional)
- **Purpose**: System alerts and notifications
- **Key Fields**: Severity, type, acknowledgment status

## Customization

You can modify the diagram in dbdiagram.io:
- Change colors and themes
- Adjust layout
- Add notes and annotations
- Export as PNG, PDF, or SQL

## Export Options

From dbdiagram.io, you can:
- **Export as Image**: PNG or PDF
- **Export as SQL**: Generate CREATE TABLE statements
- **Share Link**: Get a shareable URL
- **Embed**: Get embed code for documentation
