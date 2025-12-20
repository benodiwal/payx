# Testing

## Running Tests

Tests use [testcontainers](https://github.com/testcontainers/testcontainers-rs) to automatically spin up an isolated PostgreSQL container. No external database setup is required—just Docker.

```bash
# Run all tests
cargo test -p payx-server -- --test-threads=1

# Run a specific test
cargo test -p payx-server -- test_transfer_between_accounts --test-threads=1

# Run with output
cargo test -p payx-server -- --test-threads=1 --nocapture
```

**Important:** Tests must run with `--test-threads=1` because they share a database container and each test truncates tables during setup.

## Test Coverage

The test suite includes 25 integration tests covering:

### Transaction Tests
- `test_transfer_between_accounts` - Transfer funds between two accounts
- `test_credit_transaction` - Add funds to an account
- `test_debit_transaction` - Withdraw funds from an account
- `test_idempotency` - Duplicate requests return same result

### Error Handling Tests
- `test_insufficient_funds` - Reject transactions exceeding balance
- `test_account_not_found` - 404 for non-existent accounts
- `test_invalid_uuid` - 400 for malformed UUIDs
- `test_malformed_json` - 400 for invalid JSON
- `test_missing_required_fields` - 422 for incomplete requests

### Authentication Tests
- `test_missing_api_key` - 401 without Authorization header
- `test_invalid_api_key` - 401 with wrong API key
- `test_malformed_authorization_header` - 401 for bad header format

### CRUD Tests
- `test_create_business` - Create business and receive API key
- `test_get_business` - Retrieve business details
- `test_list_businesses` - List all businesses
- `test_duplicate_business_email` - Reject duplicate emails
- `test_create_account` - Create account with initial balance
- `test_list_accounts` - List accounts
- `test_list_transactions` - List transactions
- `test_get_transaction` - Get transaction details

### Webhook Tests
- `test_webhook_outbox_created_on_transaction` - Verify webhook queued
- `test_list_webhook_deliveries` - List webhook delivery attempts

### Concurrency Tests
- `test_concurrent_transfers_preserve_balance` - 10 parallel transfers maintain integrity
- `test_concurrent_debits_respect_balance` - Parallel debits respect balance limits

### Health Tests
- `test_health_endpoints` - `/health` and `/ready` endpoints

## Test Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Test Suite                        │
├─────────────────────────────────────────────────────┤
│  setup()                                            │
│    ├── get_test_db() → Testcontainers PostgreSQL   │
│    ├── CREATE EXTENSION pgcrypto                    │
│    ├── Run migrations                               │
│    ├── TRUNCATE all tables                          │
│    └── Return (Router, Pool)                        │
├─────────────────────────────────────────────────────┤
│  Each test:                                         │
│    ├── Calls setup()                                │
│    ├── Creates test data via API                    │
│    ├── Performs assertions                          │
│    └── Data cleaned up by next test's TRUNCATE     │
└─────────────────────────────────────────────────────┘
```

## Adding New Tests

1. Add test function to `crates/payx-server/tests/transfer_test.rs`
2. Use `setup().await` to get router and pool
3. Use helper functions: `create_business()`, `create_account()`, `get_balance()`

Example:

```rust
#[tokio::test]
async fn test_my_feature() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "1000.00").await;

    // Make API request
    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/accounts/{}", account_id))
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}
```
