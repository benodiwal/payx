# Observability

## Logging

### Format

All logs are JSON-formatted for easy parsing:

```json
{
  "timestamp": "2024-12-17T10:00:00.000Z",
  "level": "INFO",
  "target": "payx::api::handlers::transactions",
  "message": "transaction completed",
  "span": {
    "request_id": "req_abc123"
  }
}
```

### Log Levels

| Level | Usage |
|-------|-------|
| `ERROR` | Failures requiring attention |
| `WARN` | Recoverable issues |
| `INFO` | Business events, startup/shutdown |
| `DEBUG` | Detailed operational info |
| `TRACE` | Very detailed debugging |

### Configuration

```bash
# Set log level
RUST_LOG=info

# Multiple targets
RUST_LOG=info,tower_http=debug,payx=debug

# Specific module
RUST_LOG=payx::api::handlers=trace
```

## Distributed Tracing

### OpenTelemetry

PayX exports traces via OTLP (OpenTelemetry Protocol).

```bash
# Enable tracing
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
```

### Grafana Tempo

Traces are collected by [Grafana Tempo](https://grafana.com/oss/tempo/) and visualized in Grafana:

```bash
# Start all services (via Docker Compose)
docker compose up -d

# Open Grafana
open http://localhost:3000
```

**Viewing traces:**
1. Login to Grafana (admin/admin)
2. Go to **Explore** (compass icon)
3. Select **Tempo** datasource
4. Search by Trace ID or use TraceQL queries

### Trace Structure

```
HTTP Request
└── payx::api::handlers::transactions::create
    ├── sqlx::query (check idempotency)
    ├── sqlx::query (lock accounts)
    ├── sqlx::query (update balances)
    ├── sqlx::query (insert transaction)
    ├── sqlx::query (insert ledger entries)
    └── sqlx::query (insert webhook outbox)
```

## Request ID

Every request gets a unique ID for correlation:

- Header: `X-Request-Id`
- Auto-generated if not provided
- Propagated to downstream services
- Included in all log entries

## Key Metrics

### Business Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `transactions_total` | Counter | Total transactions by type and status |
| `transaction_amount` | Histogram | Transaction amounts |
| `accounts_total` | Counter | Total accounts created |

### Operational Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `http_requests_total` | Counter | Requests by endpoint and status |
| `http_request_duration_seconds` | Histogram | Request latency |
| `db_connections_active` | Gauge | Active database connections |
| `webhook_deliveries_total` | Counter | Webhook deliveries by status |
| `webhook_delivery_latency_seconds` | Histogram | Webhook delivery time |
| `rate_limit_exceeded_total` | Counter | Rate limit violations |

## Alerting Suggestions

### Critical

- Error rate > 5% for 5 minutes
- P99 latency > 5 seconds
- Database connection failures
- Webhook delivery backlog > 1000

### Warning

- Error rate > 1% for 10 minutes
- P95 latency > 2 seconds
- Rate limit exceeded rate high
- Webhook failure rate > 10%

## Debugging Tips

### Request Issues

1. Find request ID from client or logs
2. Search logs: `request_id=<id>`
3. View trace in Grafana (Explore → Tempo)

### Transaction Issues

1. Get transaction ID
2. Query ledger entries: `SELECT * FROM ledger_entries WHERE transaction_id = ?`
3. Verify balances sum to zero (double-entry)

### Webhook Issues

1. List failed webhooks via CLI:
   ```bash
   payx webhook list --status failed
   ```
2. Get details of a specific delivery:
   ```bash
   payx webhook get <delivery-id>
   ```
3. Retry a failed webhook:
   ```bash
   payx webhook retry <delivery-id>
   ```
4. Or query database directly:
   ```sql
   SELECT * FROM webhook_outbox WHERE status != 'delivered'
   ```
5. View `last_error` for failure reason
6. Check target server logs
