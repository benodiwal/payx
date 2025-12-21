# Quick Start

## Using Docker Compose

The fastest way to get started:

```bash
# Start all services (app, postgres, tempo, grafana)
docker compose up -d

# Check logs
docker compose logs -f app

# View traces in Grafana
open http://localhost:3000
```

Services:
- **API**: http://localhost:8080
- **Grafana**: http://localhost:3000 (admin/admin)
- **Tempo**: http://localhost:3200 (trace backend)
- **PostgreSQL**: localhost:5432

## Local Development

```bash
# Start postgres only
docker compose up -d db

# Copy environment file
cp .env.example .env

# Run the server
cargo run -p payx-server

# Run tests
cargo test -p payx-server

# Build the CLI
cargo build -p payx-cli
```

## First API Call

### 1. Create a Business

```bash
curl -X POST http://localhost:8080/v1/businesses \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Acme Corp",
    "email": "admin@acme.com"
  }'
```

Response:
```json
{
  "business": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Acme Corp",
    "email": "admin@acme.com"
  },
  "api_key": {
    "id": "...",
    "key": "payx_abc123...",
    "prefix": "payx_abc123"
  },
  "webhook_secret": "whsec_..."
}
```

> **Important**: Save the `api_key.key` and `webhook_secret`. They cannot be retrieved later.

### 2. Create Accounts

```bash
# Source account with initial balance
curl -X POST http://localhost:8080/v1/accounts \
  -H "Authorization: Bearer payx_YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "business_id": "BUSINESS_UUID",
    "currency": "USD",
    "initial_balance": "1000.00"
  }'

# Destination account
curl -X POST http://localhost:8080/v1/accounts \
  -H "Authorization: Bearer payx_YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "business_id": "BUSINESS_UUID",
    "currency": "USD"
  }'
```

### 3. Execute a Transfer

```bash
curl -X POST http://localhost:8080/v1/transactions \
  -H "Authorization: Bearer payx_YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: transfer-001" \
  -d '{
    "type": "transfer",
    "source_account_id": "SOURCE_UUID",
    "destination_account_id": "DEST_UUID",
    "amount": "100.00",
    "currency": "USD"
  }'
```

### 4. Check Balances

```bash
curl http://localhost:8080/v1/accounts/ACCOUNT_UUID \
  -H "Authorization: Bearer payx_YOUR_API_KEY"
```

## Using the CLI

You can also use the CLI for these operations:

```bash
# Configure the CLI
payx config set --api-key payx_YOUR_API_KEY

# Create accounts
payx account create --business-id BUSINESS_UUID --balance 1000 --currency USD

# Transfer funds
payx transaction transfer \
  --from SOURCE_UUID \
  --to DEST_UUID \
  --amount 100 \
  --currency USD

# Check balance
payx account get ACCOUNT_UUID
```

See [CLI Tool](./cli.md) for full documentation.
