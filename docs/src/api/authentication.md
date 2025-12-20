# Authentication

All API endpoints (except health checks) require authentication via Bearer token.

## Header Format

```
Authorization: Bearer <api_key>
```

## API Key Format

API keys are generated when creating a business:

```
payx_<base64 encoded random bytes>
```

Example: `payx_abc123XYZ789...`

## Key Storage

- Keys are hashed with Argon2 before storage
- Only the key prefix (first 12 characters) is stored in plaintext for lookup
- Full keys cannot be retrieved after creation

## Key Lifecycle

| State | Description |
|-------|-------------|
| Active | Key can be used for API requests |
| Expired | Key has passed its `expires_at` timestamp |
| Revoked | Key has been manually revoked |

## Rate Limiting

Each API key has an associated rate limit (default: 100 requests/minute).

When exceeded:
- Status: `429 Too Many Requests`
- Error code: `rate_limit_exceeded`

## Example Request

```bash
curl http://localhost:8080/v1/accounts/123 \
  -H "Authorization: Bearer payx_abc123XYZ789..."
```

## Common Errors

| Status | Code | Description |
|--------|------|-------------|
| 401 | `invalid_api_key` | Missing, malformed, expired, or revoked key |
| 429 | `rate_limit_exceeded` | Too many requests |
