# Configuration

All configuration is done via environment variables.

## Required Variables

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string |

## Optional Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND_ADDRESS` | `0.0.0.0:8080` | Server bind address |
| `DB_MAX_CONNECTIONS` | `20` | Database connection pool size |
| `RATE_LIMIT_PER_MINUTE` | `100` | Default rate limit per API key |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | - | OpenTelemetry collector endpoint |
| `RUST_LOG` | `info` | Log level filter |

## Example .env File

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/payx
BIND_ADDRESS=0.0.0.0:8080
DB_MAX_CONNECTIONS=20
RATE_LIMIT_PER_MINUTE=100
RUST_LOG=info,tower_http=debug,payx=debug

# Optional: OpenTelemetry
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
```

## Docker Compose Environment

When using Docker Compose, environment variables are set in `docker-compose.yml`:

```yaml
services:
  app:
    environment:
      - DATABASE_URL=postgres://postgres:postgres@db:5432/payx
      - BIND_ADDRESS=0.0.0.0:8080
      - RUST_LOG=info,tower_http=debug,payx=debug
      - OTEL_EXPORTER_OTLP_ENDPOINT=http://tempo:4317
```
