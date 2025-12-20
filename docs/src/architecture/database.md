# Database Schema

## Entity Relationship

```
businesses 1──┬──N accounts
              │
              └──N api_keys
              │
              └──N webhook_outbox

accounts N──┬──N transactions
            │
            └──N ledger_entries

api_keys 1──N rate_limit_windows
```

## Tables

### businesses

Business entities that own accounts.

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `name` | VARCHAR(255) | Business name |
| `email` | VARCHAR(255) | Unique email |
| `webhook_url` | TEXT | Webhook delivery URL |
| `webhook_secret` | TEXT | HMAC signing secret |
| `created_at` | TIMESTAMPTZ | Creation timestamp |
| `updated_at` | TIMESTAMPTZ | Last update timestamp |

### accounts

Financial accounts with balances.

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `business_id` | UUID | Foreign key to businesses |
| `account_type` | VARCHAR(50) | Account type (default: checking) |
| `currency` | VARCHAR(3) | ISO 4217 currency code |
| `balance` | DECIMAL(19,4) | Current balance |
| `available_balance` | DECIMAL(19,4) | Available for transactions |
| `version` | BIGINT | Optimistic locking version |
| `created_at` | TIMESTAMPTZ | Creation timestamp |
| `updated_at` | TIMESTAMPTZ | Last update timestamp |

**Constraints:**
- `balance >= 0`
- `available_balance >= 0`

### transactions

Immutable transaction log.

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `idempotency_key` | VARCHAR(255) | Unique idempotency key |
| `type` | VARCHAR(20) | credit, debit, transfer |
| `status` | VARCHAR(20) | pending, completed, failed |
| `source_account_id` | UUID | Account debited |
| `destination_account_id` | UUID | Account credited |
| `amount` | DECIMAL(19,4) | Transaction amount |
| `currency` | VARCHAR(3) | Transaction currency |
| `description` | TEXT | Optional description |
| `metadata` | JSONB | Custom metadata |
| `created_at` | TIMESTAMPTZ | Creation timestamp |
| `completed_at` | TIMESTAMPTZ | Completion timestamp |

**Constraints:**
- `amount > 0`
- Unique index on `idempotency_key` (where not null)

### ledger_entries

Double-entry bookkeeping entries.

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `transaction_id` | UUID | Foreign key to transactions |
| `account_id` | UUID | Foreign key to accounts |
| `entry_type` | VARCHAR(10) | debit or credit |
| `amount` | DECIMAL(19,4) | Entry amount |
| `balance_after` | DECIMAL(19,4) | Account balance after entry |
| `created_at` | TIMESTAMPTZ | Creation timestamp |

### api_keys

API key storage with secure hashing.

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `business_id` | UUID | Foreign key to businesses |
| `key_hash` | TEXT | Argon2 hash of full key |
| `key_prefix` | VARCHAR(12) | First 12 chars for lookup |
| `name` | VARCHAR(255) | Optional key name |
| `rate_limit_per_minute` | INT | Rate limit (default: 100) |
| `created_at` | TIMESTAMPTZ | Creation timestamp |
| `expires_at` | TIMESTAMPTZ | Optional expiration |
| `revoked_at` | TIMESTAMPTZ | Revocation timestamp |
| `last_used_at` | TIMESTAMPTZ | Last usage timestamp |

### webhook_outbox

Transactional outbox for reliable webhook delivery.

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `business_id` | UUID | Foreign key to businesses |
| `event_type` | VARCHAR(100) | Event type |
| `payload` | JSONB | Event payload |
| `status` | VARCHAR(20) | pending, retrying, delivered, failed |
| `attempts` | INT | Delivery attempts |
| `max_attempts` | INT | Maximum attempts (default: 5) |
| `next_attempt_at` | TIMESTAMPTZ | Next delivery attempt |
| `last_error` | TEXT | Last error message |
| `created_at` | TIMESTAMPTZ | Creation timestamp |
| `processed_at` | TIMESTAMPTZ | Delivery timestamp |

### rate_limit_windows

Rate limiting counters.

| Column | Type | Description |
|--------|------|-------------|
| `api_key_id` | UUID | Foreign key to api_keys |
| `window_start` | TIMESTAMPTZ | Window start timestamp |
| `request_count` | INT | Requests in window |

Primary key: `(api_key_id, window_start)`

## Indexes

| Index | Table | Columns | Purpose |
|-------|-------|---------|---------|
| `idx_transactions_idempotency` | transactions | idempotency_key | Fast idempotency lookup |
| `idx_accounts_business` | accounts | business_id | Account queries by business |
| `idx_api_keys_prefix` | api_keys | key_prefix | O(1) API key lookup |
| `idx_webhook_outbox_pending` | webhook_outbox | status, next_attempt_at | Efficient polling |
| `idx_ledger_entries_transaction` | ledger_entries | transaction_id | Entry lookup |
| `idx_ledger_entries_account` | ledger_entries | account_id | Account history |
