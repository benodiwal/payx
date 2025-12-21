use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use payx_server::config::Config;
use payx_server::App;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;
use tower::ServiceExt;

static TEST_CONTAINER: OnceCell<Arc<ContainerAsync<Postgres>>> = OnceCell::const_new();
static TEST_POOL: OnceCell<PgPool> = OnceCell::const_new();

async fn get_test_db() -> (PgPool, String) {
    let container = TEST_CONTAINER
        .get_or_init(|| async {
            let container = Postgres::default()
                .start()
                .await
                .expect("Failed to start postgres container");
            Arc::new(container)
        })
        .await;

    let host = container.get_host().await.expect("Failed to get host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get port");
    let database_url = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

    let pool = TEST_POOL
        .get_or_init(|| async {
            let pool = sqlx::PgPool::connect(&database_url)
                .await
                .expect("Failed to connect to test database");

            // Enable pgcrypto extension for gen_random_uuid()
            sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
                .execute(&pool)
                .await
                .expect("Failed to create pgcrypto extension");

            pool
        })
        .await
        .clone();

    (pool, database_url)
}

async fn setup() -> (Router, PgPool) {
    let (_pool, database_url) = get_test_db().await;

    let config = Config {
        database_url,
        bind_address: "0.0.0.0:8080".to_string(),
        db_max_connections: 5,
        otlp_endpoint: None,
        rate_limit_per_minute: 1000,
    };

    let app = App::new(config).await.expect("Failed to create app");
    let pool = app.db().clone();

    sqlx::query("TRUNCATE businesses, accounts, transactions, ledger_entries, api_keys, webhook_outbox, rate_limit_windows CASCADE")
        .execute(&pool)
        .await
        .ok();

    (app.router(), pool)
}

async fn create_business(router: &Router) -> (String, String) {
    create_business_with_webhook(router, None).await
}

async fn create_business_with_webhook(
    router: &Router,
    webhook_url: Option<&str>,
) -> (String, String) {
    let mut body = json!({
        "name": "Test Business",
        "email": format!("test{}@example.com", uuid::Uuid::new_v4())
    });

    if let Some(url) = webhook_url {
        body["webhook_url"] = json!(url);
    }

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/businesses")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    (
        json["business"]["id"].as_str().unwrap().to_string(),
        json["api_key"]["key"].as_str().unwrap().to_string(),
    )
}

async fn create_account(
    router: &Router,
    api_key: &str,
    business_id: &str,
    initial_balance: &str,
) -> String {
    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/accounts")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "business_id": business_id,
                        "currency": "USD",
                        "initial_balance": initial_balance
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["id"].as_str().unwrap().to_string()
}

async fn get_balance(router: &Router, api_key: &str, account_id: &str) -> String {
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

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    json["balance"].as_str().unwrap().to_string()
}

// =============================================================================
// TRANSFER TESTS
// =============================================================================

#[tokio::test]
async fn test_transfer_between_accounts() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let source_id = create_account(&router, &api_key, &business_id, "1000.00").await;
    let dest_id = create_account(&router, &api_key, &business_id, "0.00").await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .header("idempotency-key", "test-transfer-001")
                .body(Body::from(
                    json!({
                        "type": "transfer",
                        "source_account_id": source_id,
                        "destination_account_id": dest_id,
                        "amount": "250.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let source_balance = get_balance(&router, &api_key, &source_id).await;
    let dest_balance = get_balance(&router, &api_key, &dest_id).await;

    assert_eq!(source_balance, "750.0000");
    assert_eq!(dest_balance, "250.0000");
}

#[tokio::test]
async fn test_credit_transaction() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "100.00").await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "credit",
                        "destination_account_id": account_id,
                        "amount": "500.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let balance = get_balance(&router, &api_key, &account_id).await;
    assert_eq!(balance, "600.0000");
}

#[tokio::test]
async fn test_debit_transaction() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "500.00").await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "debit",
                        "source_account_id": account_id,
                        "amount": "200.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let balance = get_balance(&router, &api_key, &account_id).await;
    assert_eq!(balance, "300.0000");
}

// =============================================================================
// IDEMPOTENCY TESTS
// =============================================================================

#[tokio::test]
async fn test_idempotency() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "500.00").await;

    let make_credit = |router: Router| {
        let api_key = api_key.clone();
        let account_id = account_id.clone();
        async move {
            router
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/transactions")
                        .header("authorization", format!("Bearer {}", api_key))
                        .header("content-type", "application/json")
                        .header("idempotency-key", "idem-credit-001")
                        .body(Body::from(
                            json!({
                                "type": "credit",
                                "destination_account_id": account_id,
                                "amount": "100.00",
                                "currency": "USD"
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
        }
    };

    let res1 = make_credit(router.clone()).await.unwrap();
    assert_eq!(res1.status(), StatusCode::CREATED);
    let body1 = axum::body::to_bytes(res1.into_body(), usize::MAX)
        .await
        .unwrap();
    let txn1: Value = serde_json::from_slice(&body1).unwrap();

    let res2 = make_credit(router.clone()).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body2 = axum::body::to_bytes(res2.into_body(), usize::MAX)
        .await
        .unwrap();
    let txn2: Value = serde_json::from_slice(&body2).unwrap();

    assert_eq!(txn1["id"], txn2["id"]);

    let balance = get_balance(&router, &api_key, &account_id).await;
    assert_eq!(balance, "600.0000");
}

// =============================================================================
// ERROR HANDLING TESTS
// =============================================================================

#[tokio::test]
async fn test_insufficient_funds() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "50.00").await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "debit",
                        "source_account_id": account_id,
                        "amount": "100.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        error["error"]["code"].as_str().unwrap(),
        "insufficient_funds"
    );
}

#[tokio::test]
async fn test_account_not_found() {
    let (router, _pool) = setup().await;

    let (_business_id, api_key) = create_business(&router).await;
    let fake_account_id = uuid::Uuid::new_v4();

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/accounts/{}", fake_account_id))
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_invalid_uuid() {
    let (router, _pool) = setup().await;

    let (_business_id, api_key) = create_business(&router).await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/accounts/not-a-uuid")
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_malformed_json() {
    let (router, _pool) = setup().await;

    let (_business_id, api_key) = create_business(&router).await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from("{ invalid json }"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_missing_required_fields() {
    let (router, _pool) = setup().await;

    let (_business_id, api_key) = create_business(&router).await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "credit"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// =============================================================================
// AUTHENTICATION TESTS
// =============================================================================

#[tokio::test]
async fn test_missing_api_key() {
    let (router, _pool) = setup().await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"].as_str().unwrap(), "invalid_api_key");
}

#[tokio::test]
async fn test_invalid_api_key() {
    let (router, _pool) = setup().await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/accounts")
                .header("authorization", "Bearer invalid_key_12345")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_malformed_authorization_header() {
    let (router, _pool) = setup().await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/accounts")
                .header("authorization", "NotBearer sometoken")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// =============================================================================
// BUSINESS TESTS
// =============================================================================

#[tokio::test]
async fn test_create_business() {
    let (router, _pool) = setup().await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/businesses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Acme Corp",
                        "email": format!("acme{}@example.com", uuid::Uuid::new_v4())
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["business"]["id"].as_str().is_some());
    assert_eq!(json["business"]["name"].as_str().unwrap(), "Acme Corp");
    assert!(json["api_key"]["key"].as_str().is_some());
}

#[tokio::test]
async fn test_get_business() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/businesses/{}", business_id))
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"].as_str().unwrap(), business_id);
}

#[tokio::test]
async fn test_list_businesses() {
    let (router, _pool) = setup().await;

    let (_business_id, api_key) = create_business(&router).await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/businesses")
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().is_some());
}

#[tokio::test]
async fn test_duplicate_business_email() {
    let (router, _pool) = setup().await;

    let email = format!("duplicate{}@example.com", uuid::Uuid::new_v4());

    let res1 = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/businesses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "First Business",
                        "email": email
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res1.status(), StatusCode::CREATED);

    let res2 = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/businesses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Second Business",
                        "email": email
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Database constraint violation returns 500 (DB error)
    assert_eq!(res2.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// =============================================================================
// ACCOUNT TESTS
// =============================================================================

#[tokio::test]
async fn test_create_account() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/accounts")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "business_id": business_id,
                        "currency": "USD",
                        "initial_balance": "1000.00"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["id"].as_str().is_some());
    assert_eq!(json["currency"].as_str().unwrap(), "USD");
    assert_eq!(json["balance"].as_str().unwrap(), "1000.0000");
}

#[tokio::test]
async fn test_list_accounts() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    create_account(&router, &api_key, &business_id, "100.00").await;
    create_account(&router, &api_key, &business_id, "200.00").await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/accounts")
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let accounts = json.as_array().unwrap();
    assert_eq!(accounts.len(), 2);
}

// =============================================================================
// TRANSACTION LIST TESTS
// =============================================================================

#[tokio::test]
async fn test_list_transactions() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "1000.00").await;

    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "credit",
                        "destination_account_id": account_id,
                        "amount": "100.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(!json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_get_transaction() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "1000.00").await;

    let create_res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "credit",
                        "destination_account_id": account_id,
                        "amount": "100.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(create_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let txn: Value = serde_json::from_slice(&body).unwrap();
    let txn_id = txn["id"].as_str().unwrap();

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/transactions/{}", txn_id))
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"].as_str().unwrap(), txn_id);
}

// =============================================================================
// WEBHOOK TESTS
// =============================================================================

#[tokio::test]
async fn test_webhook_outbox_created_on_transaction() {
    let (router, pool) = setup().await;

    let (business_id, api_key) =
        create_business_with_webhook(&router, Some("https://example.com/webhook")).await;
    let account_id = create_account(&router, &api_key, &business_id, "1000.00").await;

    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "credit",
                        "destination_account_id": account_id,
                        "amount": "100.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let webhook_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM webhook_outbox WHERE business_id = $1")
            .bind(uuid::Uuid::parse_str(&business_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(webhook_count.0 >= 1);
}

#[tokio::test]
async fn test_list_webhook_deliveries() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) =
        create_business_with_webhook(&router, Some("https://example.com/webhook")).await;
    let account_id = create_account(&router, &api_key, &business_id, "1000.00").await;

    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/transactions")
                .header("authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "type": "credit",
                        "destination_account_id": account_id,
                        "amount": "100.00",
                        "currency": "USD"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/webhooks/deliveries")
                .header("authorization", format!("Bearer {}", api_key))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().is_some());
}

// =============================================================================
// HEALTH CHECK TESTS
// =============================================================================

#[tokio::test]
async fn test_health_endpoints() {
    let (router, _pool) = setup().await;

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

// =============================================================================
// CONCURRENT TRANSFER TESTS
// =============================================================================

#[tokio::test]
async fn test_concurrent_transfers_preserve_balance() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let source_id = create_account(&router, &api_key, &business_id, "1000.00").await;
    let dest_id = create_account(&router, &api_key, &business_id, "0.00").await;

    let mut handles = vec![];

    for i in 0..10 {
        let router = router.clone();
        let api_key = api_key.clone();
        let source_id = source_id.clone();
        let dest_id = dest_id.clone();

        let handle = tokio::spawn(async move {
            router
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/transactions")
                        .header("authorization", format!("Bearer {}", api_key))
                        .header("content-type", "application/json")
                        .header("idempotency-key", format!("concurrent-transfer-{}", i))
                        .body(Body::from(
                            json!({
                                "type": "transfer",
                                "source_account_id": source_id,
                                "destination_account_id": dest_id,
                                "amount": "10.00",
                                "currency": "USD"
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(res)) = handle.await {
            if res.status() == StatusCode::CREATED || res.status() == StatusCode::OK {
                success_count += 1;
            }
        }
    }

    assert_eq!(success_count, 10);

    let source_balance = get_balance(&router, &api_key, &source_id).await;
    let dest_balance = get_balance(&router, &api_key, &dest_id).await;

    assert_eq!(source_balance, "900.0000");
    assert_eq!(dest_balance, "100.0000");
}

#[tokio::test]
async fn test_concurrent_debits_respect_balance() {
    let (router, _pool) = setup().await;

    let (business_id, api_key) = create_business(&router).await;
    let account_id = create_account(&router, &api_key, &business_id, "100.00").await;

    let mut handles = vec![];

    // Try to debit 50 five times (250 total) from account with 100
    for i in 0..5 {
        let router = router.clone();
        let api_key = api_key.clone();
        let account_id = account_id.clone();

        let handle = tokio::spawn(async move {
            router
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/transactions")
                        .header("authorization", format!("Bearer {}", api_key))
                        .header("content-type", "application/json")
                        .header("idempotency-key", format!("concurrent-debit-{}", i))
                        .body(Body::from(
                            json!({
                                "type": "debit",
                                "source_account_id": account_id,
                                "amount": "50.00",
                                "currency": "USD"
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    let mut failure_count = 0;

    for handle in handles {
        if let Ok(Ok(res)) = handle.await {
            match res.status() {
                StatusCode::CREATED | StatusCode::OK => success_count += 1,
                StatusCode::UNPROCESSABLE_ENTITY => failure_count += 1,
                _ => {}
            }
        }
    }

    // Only 2 debits of 50 should succeed (100 balance)
    assert_eq!(success_count, 2);
    assert_eq!(failure_count, 3);

    let balance = get_balance(&router, &api_key, &account_id).await;
    // Balance can be "0.0000" or "0" depending on decimal formatting
    assert!(balance == "0.0000" || balance == "0");
}
