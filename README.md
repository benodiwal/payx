# PayX

A transaction service in Rust with double-entry bookkeeping.

## Features

- API key auth (Argon2)
- Atomic transfers with ledger entries
- Webhooks (outbox pattern)
- Idempotency
- Rate limiting
- OpenTelemetry

## Project Structure

```
payx/
├── crates/
│   ├── payx-server/    # API server
│   └── payx-cli/       # CLI
├── docs/               # mdbook docs
└── docker-compose.yml
```

See [DESIGN.md](./DESIGN.md) for architecture details.

## Quick Start

```bash
docker compose up -d
```

- **API**: http://localhost:8080
- **Grafana**: http://localhost:3000 (admin/admin)

## Documentation

Full documentation is available in the `docs/` directory.

```bash
# Install mdbook
cargo install mdbook

# Serve documentation locally
cd docs && mdbook serve
```

Then open http://localhost:3000

## CLI Tool

```bash
# Build and install the CLI
cargo install --path crates/payx-cli

# Configure
payx config set --server http://localhost:8080
payx config set --api-key <your_api_key>

# Create an account
payx account create --business-id <id> --balance 1000

# Transfer funds
payx transaction transfer --from <source> --to <dest> --amount 100 --currency USD
```

## API Usage

```bash
# Create a business (returns API key)
curl -X POST http://localhost:8080/v1/businesses \
  -H "Content-Type: application/json" \
  -d '{"name": "Acme", "email": "admin@acme.com"}'

# Create an account
curl -X POST http://localhost:8080/v1/accounts \
  -H "Authorization: Bearer <api_key>" \
  -H "Content-Type: application/json" \
  -d '{"business_id": "<id>", "currency": "USD", "initial_balance": "1000.00"}'

# Transfer funds
curl -X POST http://localhost:8080/v1/transactions \
  -H "Authorization: Bearer <api_key>" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: txn-001" \
  -d '{
    "type": "transfer",
    "source_account_id": "<source>",
    "destination_account_id": "<dest>",
    "amount": "100.00",
    "currency": "USD"
  }'
```

## Development

```bash
# Start database
docker compose up -d db

# Run server
cp .env.example .env
cargo run -p payx-server
```

## Testing

Tests use [testcontainers](https://github.com/testcontainers/testcontainers-rs) for database isolation.

```bash
cargo test -p payx-server -- --test-threads=1
```

Note: `--test-threads=1` required (shared database container).

## License

Apache-2.0