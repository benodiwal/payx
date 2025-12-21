# Deployment

## Docker Compose (Development)

The simplest deployment for development and testing:

```bash
docker compose up -d
```

Services:
- **app**: PayX API server
- **db**: PostgreSQL 16
- **tempo**: Distributed tracing
- **loki**: Log aggregation
- **promtail**: Log collector
- **grafana**: Observability UI (http://localhost:3000)

## Docker (Production)

### Build

```bash
docker build -t payx:latest .
```

### Run

```bash
docker run -d \
  -p 8080:8080 \
  -e DATABASE_URL=postgres://user:pass@host:5432/payx \
  -e RUST_LOG=info \
  payx:latest
```

## Kubernetes

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: payx
spec:
  replicas: 3
  selector:
    matchLabels:
      app: payx
  template:
    metadata:
      labels:
        app: payx
    spec:
      containers:
      - name: payx
        image: payx:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: payx-secrets
              key: database-url
        - name: RUST_LOG
          value: "info"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: payx
spec:
  selector:
    app: payx
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
```

## Health Checks

| Endpoint | Purpose | Response |
|----------|---------|----------|
| `GET /health` | Liveness probe | `200 OK` if running |
| `GET /ready` | Readiness probe | `200 OK` if DB connected |

### Liveness

Checks if the server is running. Failure triggers restart.

### Readiness

Checks if the server can handle requests. Failure removes from load balancer.

## Database Migrations

Migrations run automatically on startup via `sqlx::migrate!()`.

For manual migration:

```bash
# Install sqlx-cli
cargo install sqlx-cli

# Run migrations
sqlx migrate run --database-url $DATABASE_URL
```

## Graceful Shutdown

The server handles `SIGTERM` and `SIGINT`:

1. Stop accepting new connections
2. Complete in-flight requests (30s timeout)
3. Wait for webhook processor to finish current batch
4. Flush OpenTelemetry data
5. Exit

## Security Checklist

- [ ] Use HTTPS in production (terminate at load balancer)
- [ ] Set strong database password
- [ ] Restrict database network access
- [ ] Enable connection pooling limits
- [ ] Configure rate limiting appropriately
- [ ] Rotate API keys periodically
- [ ] Monitor for suspicious activity
