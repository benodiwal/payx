use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use payx_server::config::Config;
use payx_server::App;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

async fn setup() -> (Router, PgPool) {
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("config");
    let app = App::new(config).await.expect("app");
    let pool = app.db().clone();

    sqlx::query("TRUNCATE businesses, accounts, transactions, ledger_entries, api_keys, webhook_outbox, rate_limit_windows CASCADE")
        .execute(&pool)
        .await
        .ok();

    (app.router(), pool)
}

async fn create_business(router: &Router) -> (String, String) {
    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/businesses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Test Business",
                        "email": format!("test{}@example.com", uuid::Uuid::new_v4())
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
