# Observability

PayX includes a complete observability stack with distributed tracing and centralized logging.

## Observability Stack

| Service | Port | Purpose |
|---------|------|---------|
| Grafana | 3000 | Unified UI for traces and logs |
| Tempo | 3200 | Distributed trace storage |
| Loki | 3100 | Log aggregation |
| Promtail | - | Log collector for Docker containers |

```bash
# Start all services
docker compose up -d

# Open Grafana
open http://localhost:3000  # admin/admin
```

## Logging

### Log Aggregation with Loki

Logs are collected by Promtail from Docker containers and stored in Loki.

**Viewing logs in Grafana:**
1. Go to **Explore** (compass icon)
2. Select **Loki** datasource
3. Use LogQL queries:

```logql
# All app logs
{service="app"}

# Filter by log level
{service="app"} | json | level="error"

# Search for specific text
{service="app"} |= "transaction"

# Filter POST requests
{service="app"} |= "POST"
```

**Available labels:**
- `service` - Docker Compose service name
- `container` - Container name
- `level` - Log level (info, error, etc.)
- `target` - Rust module path

### Log Format

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

Traces are collected by [Grafana Tempo](https://grafana.com/oss/tempo/) and visualized in Grafana.

**Viewing traces:**
1. Go to **Explore** (compass icon)
2. Select **Tempo** datasource
3. Use the **Search** tab to find traces
4. Or query by Trace ID directly

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

## Trace-Log Correlation

Traces and logs are linked automatically:

- **Traces → Logs**: Click a trace in Tempo to see related logs in Loki
- **Logs → Traces**: If a log contains a `trace_id`, click it to view the full trace

This correlation is configured in the Grafana datasources and enables seamless debugging across both views.

## Retention Settings

| Data | Retention | Configuration |
|------|-----------|---------------|
| Traces (Tempo) | 3 days | `docker/tempo/tempo.yaml` → `block_retention` |
| Logs (Loki) | 7 days (ingestion) | `docker/loki/loki.yaml` → `reject_old_samples_max_age` |

To modify retention, update the respective configuration files and restart the services:

```bash
docker compose restart tempo loki
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
