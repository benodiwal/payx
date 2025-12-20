# Transaction Processing

## Atomic Balance Updates

All balance updates use PostgreSQL transactions with row-level locking.

### Transfer Flow

```
1. Begin transaction
2. Check idempotency key
3. Lock accounts (consistent order)
4. Validate balances
5. Update account balances
6. Insert transaction record
7. Insert ledger entries
8. Insert webhook outbox event
9. Commit transaction
```

### Deadlock Prevention

Accounts are locked in UUID order to prevent deadlocks:

```rust
let (first_id, second_id) = if source_id < dest_id {
    (source_id, dest_id)
} else {
    (dest_id, source_id)
};

// Lock in consistent order
sqlx::query("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
    .bind(first_id)
    .fetch_one(&mut *tx)
    .await?;

sqlx::query("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
    .bind(second_id)
    .fetch_one(&mut *tx)
    .await?;
```

## Double-Entry Bookkeeping

Every transaction creates balanced ledger entries:

| Transaction Type | Debit Account | Credit Account |
|-----------------|---------------|----------------|
| Credit | - | Destination |
| Debit | Source | - |
| Transfer | Source | Destination |

### Example: $100 Transfer

```
Transaction: A → B, $100

Ledger Entries:
┌──────────────┬───────────┬────────┬──────────────┐
│ Account      │ Type      │ Amount │ Balance After│
├──────────────┼───────────┼────────┼──────────────┤
│ A            │ debit     │ 100.00 │ 900.00       │
│ B            │ credit    │ 100.00 │ 100.00       │
└──────────────┴───────────┴────────┴──────────────┘
```

## Idempotency

Idempotency keys prevent duplicate transactions.

### Implementation

1. Unique index on `transactions.idempotency_key`
2. Check before processing:

```rust
if let Some(ref key) = idempotency_key {
    if let Some(existing) = find_by_idempotency_key(&state, key).await? {
        return Ok((StatusCode::OK, Json(existing)));
    }
}
```

### Behavior

| Scenario | Response |
|----------|----------|
| First request | `201 Created` |
| Same key, same params | `200 OK` (cached response) |
| Same key, different params | Could return `409 Conflict` |

## Money Representation

All monetary values use `DECIMAL(19, 4)`:

- **19 digits**: Handles amounts up to quadrillions
- **4 decimal places**: Sub-cent precision for calculations
- **Never floats**: Exact arithmetic, no rounding errors

### Rust Type

```rust
use rust_decimal::Decimal;

pub struct Account {
    pub balance: Decimal,
    pub available_balance: Decimal,
}
```

## Validation Rules

| Rule | Error |
|------|-------|
| Amount must be positive | `validation_error` |
| Source must have sufficient funds | `insufficient_funds` |
| Currencies must match | `currency_mismatch` |
| Accounts must exist | `account_not_found` |
