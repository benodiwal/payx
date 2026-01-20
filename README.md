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

## CI/CD Pipeline

### Pipeline Overview

The project uses GitHub Actions for CI/CD with the following stages:

**CI Pipeline** (`.github/workflows/ci.yml`):
| Stage | Tool | Purpose |
|-------|------|---------|
| Check | cargo check | Compile verification |
| Format | rustfmt | Code style |
| Clippy | clippy | Linting |
| CodeQL | CodeQL | SAST (Static Analysis) |
| Test | cargo test | Unit tests |
| Build | cargo build | Release binary |
| Docker | docker build | Container image |
| Trivy | Trivy | Container vulnerability scan |
| Container Test | curl | Runtime smoke test |
| DockerHub | docker push | Registry push |

**CD Pipeline** (`.github/workflows/cd.yml`):
| Stage | Tool | Purpose |
|-------|------|---------|
| Update Manifests | Kustomize | K8s manifest updates |
| DAST | Placeholder | Dynamic security testing |

### Required GitHub Secrets

Configure these secrets in your repository (Settings → Secrets and variables → Actions):

| Secret | Description | How to Get |
|--------|-------------|------------|
| `DOCKERHUB_USERNAME` | DockerHub username | Your DockerHub account username |
| `DOCKERHUB_TOKEN` | DockerHub access token | [DockerHub](https://hub.docker.com/) → Account Settings → Security → New Access Token |

### Setting Up Secrets

1. Go to your GitHub repository
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add `DOCKERHUB_USERNAME` with value `warriyohyperion`
5. Add `DOCKERHUB_TOKEN` with your DockerHub access token

### DockerHub Access Token

To create a DockerHub access token:

1. Log in to [DockerHub](https://hub.docker.com/)
2. Go to **Account Settings** → **Security**
3. Click **New Access Token**
4. Give it a description (e.g., "GitHub Actions - PayX")
5. Select **Read & Write** permissions
6. Copy the token and save it as `DOCKERHUB_TOKEN` secret

### Security Scanning Results

Security findings are available in the GitHub Security tab:
- **CodeQL**: SAST findings for code vulnerabilities
- **Trivy**: Container image vulnerabilities

## License

Apache-2.0