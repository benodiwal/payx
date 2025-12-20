# CLI Tool

Payx includes a command-line interface for interacting with the API.

## Installation

### From Source

```bash
cargo install --path crates/payx-cli
```

### From Docker

The CLI is included in the Docker image:

```bash
docker run --rm payx payx --help
```

## Configuration

Configure the CLI with your API credentials:

```bash
# Set the API endpoint
payx config set --server http://localhost:8080

# Set your API key
payx config set --api-key payx_your_api_key_here
```

Configuration file location varies by OS:
- **macOS**: `~/Library/Application Support/payx/config.toml`
- **Linux**: `~/.config/payx/config.toml`
- **Windows**: `%APPDATA%\payx\config.toml`

Use `payx config path` to find the exact location on your system.

## Commands

### Business Management

```bash
# List all businesses
payx business list
payx business list --limit 10 --offset 20

# Create a new business
payx business create --name "Acme Corp" --email "admin@acme.com"

# Get business details
payx business get <business-id>
```

### Account Management

```bash
# List all accounts
payx account list
payx account list --business-id <uuid>
payx account list --limit 10

# Create an account
payx account create --business-id <uuid> --currency USD --balance 1000

# Get account details
payx account get <account-id>

# List account transactions
payx account transactions <account-id> --limit 50
```

### Transactions

```bash
# List all transactions
payx transaction list
payx transaction list --account-id <uuid>
payx transaction list --limit 10

# Credit funds to an account
payx transaction credit \
  --to <account-id> \
  --amount 100.00 \
  --currency USD \
  --description "Initial deposit" \
  --idempotency-key "deposit-001"

# Debit funds from an account
payx transaction debit \
  --from <account-id> \
  --amount 50.00 \
  --currency USD

# Transfer between accounts
payx transaction transfer \
  --from <source-account-id> \
  --to <destination-account-id> \
  --amount 250.00 \
  --currency USD \
  --idempotency-key "transfer-001"

# Get transaction details
payx transaction get <transaction-id>
```

### Webhooks

```bash
# List all webhook deliveries
payx webhook list
payx webhook list --status failed
payx webhook list --status delivered --limit 10

# Get webhook delivery details
payx webhook get <delivery-id>

# Retry a failed webhook
payx webhook retry <delivery-id>
```

## Command Aliases

For convenience, some commands have short aliases:

| Command | Alias |
|---------|-------|
| `payx transaction` | `payx tx` |
| `payx webhook` | `payx wh` |

Examples:

```bash
# These are equivalent
payx transaction list
payx tx list

# These are equivalent
payx webhook list --status failed
payx wh list --status failed
```

## Output Formats

The CLI supports two output formats:

### Table (default)

```bash
payx account get <id>
```

```
╭──────────────────────────────────────┬──────────────────────────────────────┬──────────┬─────────┬───────────────────╮
│ id                                   │ business_id                          │ currency │ balance │ available_balance │
├──────────────────────────────────────┼──────────────────────────────────────┼──────────┼─────────┼───────────────────┤
│ 550e8400-e29b-41d4-a716-446655440000 │ 6ba7b810-9dad-11d1-80b4-00c04fd430c8 │ USD      │ 1000.00 │ 1000.00           │
╰──────────────────────────────────────┴──────────────────────────────────────┴──────────┴─────────┴───────────────────╯
```

### JSON

```bash
payx --format json account get <id>
```

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "business_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "currency": "USD",
  "balance": "1000.00",
  "available_balance": "1000.00"
}
```

## Examples

### Complete Workflow

```bash
# 1. Create a business (returns API key)
payx business create --name "Test Business" --email "test@example.com"

# 2. Configure CLI with the returned API key
payx config set --api-key payx_...

# 3. Create two accounts
SOURCE=$(payx --format json account create --business-id <bid> --balance 1000 | jq -r .id)
DEST=$(payx --format json account create --business-id <bid> | jq -r .id)

# 4. Transfer funds
payx transaction transfer --from $SOURCE --to $DEST --amount 250 --currency USD

# 5. Check balances
payx account get $SOURCE
payx account get $DEST
```

### Scripting with JSON Output

```bash
# List recent transactions and extract IDs
payx --format json account transactions <account-id> | jq '.[].id'

# Get transaction amounts
payx --format json account transactions <account-id> | jq '.[].amount'
```
