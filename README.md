# PayX

A production-grade transaction service built in Rust.

## Features

- API key authentication with Argon2 hashing
- Atomic transactions with double-entry bookkeeping
- Reliable webhooks via transactional outbox pattern
- Idempotency support
- Rate limiting
- OpenTelemetry integration
- CLI tool for interacting with the API

## Project Structure

```
payx/
├── crates/
│   ├── payx-server/    # API server
│   └── payx-cli/       # Command-line interface
├── docs/               # mdbook documentation
└── docker-compose.yml
```

## Design

See [DESIGN.md](./DESIGN.md) for detailed design documentation including:

- Assumptions and constraints
- API design decisions
- Database schema design
- Transaction processing flow
- Webhook system architecture
- Security model
- Trade-offs and rationale

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

# Run tests
cargo test -p payx-server
```

## License

Apache-2.0