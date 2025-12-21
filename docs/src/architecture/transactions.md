# Transaction Processing

## Flow

1. Begin TX → Check idempotency → Lock accounts (sorted by UUID) → Validate balance
2. Update balances → Insert transaction + ledger entries + webhook outbox → Commit

Accounts locked via `SELECT FOR UPDATE` in consistent order to prevent deadlocks.

## Double-Entry Bookkeeping

| Type | Debit | Credit |
|------|-------|--------|
| Credit | - | Destination |
| Debit | Source | - |
| Transfer | Source | Destination |

## Idempotency

- Unique index on `idempotency_key`
- First request: `201 Created`
- Same key + params: `200 OK` (cached)
- Same key + different params: `409 Conflict`

## Money

`DECIMAL(19,4)` - no floats, exact arithmetic.

## Validation

| Rule | Error |
|------|-------|
| Amount > 0 | `validation_error` |
| Sufficient funds | `insufficient_funds` |
| Currencies match | `currency_mismatch` |
| Accounts exist | `account_not_found` |
