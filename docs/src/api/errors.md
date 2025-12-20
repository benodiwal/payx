# Errors

All errors follow a consistent format.

## Error Response Format

```json
{
  "error": {
    "code": "error_code",
    "message": "Human readable message",
    "details": {}
  }
}
```

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `invalid_api_key` | 401 | Invalid, missing, expired, or revoked API key |
| `rate_limit_exceeded` | 429 | Too many requests for this API key |
| `validation_error` | 400 | Invalid request parameters |
| `account_not_found` | 404 | Account does not exist |
| `business_not_found` | 404 | Business does not exist |
| `transaction_not_found` | 404 | Transaction does not exist |
| `insufficient_funds` | 422 | Account balance too low for transaction |
| `currency_mismatch` | 400 | Transaction currency doesn't match account |
| `idempotency_conflict` | 409 | Idempotency key reused with different parameters |
| `database_error` | 500 | Database operation failed |
| `internal_error` | 500 | Unexpected server error |

## Error Examples

### Insufficient Funds

```json
{
  "error": {
    "code": "insufficient_funds",
    "message": "insufficient funds: available 50.0000, requested 100.0000",
    "details": {
      "available": "50.0000",
      "requested": "100.0000"
    }
  }
}
```

### Validation Error

```json
{
  "error": {
    "code": "validation_error",
    "message": "validation error: amount must be positive"
  }
}
```

### Rate Limit Exceeded

```json
{
  "error": {
    "code": "rate_limit_exceeded",
    "message": "rate limit exceeded"
  }
}
```

### Not Found

```json
{
  "error": {
    "code": "account_not_found",
    "message": "account not found: 550e8400-e29b-41d4-a716-446655440000"
  }
}
```
