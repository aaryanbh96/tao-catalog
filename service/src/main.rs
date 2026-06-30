mod models;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, patch, post},
    Router,
};
use models::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tao_catalog_service=info,tower_http=info".into()),
        )
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set (put it in a .env file next to the binary)");

    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&db_url)
        .await?;
    tracing::info!("connected to database");

    let ping_db = db.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60 * 60 * 12));
        loop {
            tick.tick().await;
            match sqlx::query("SELECT 1").execute(&ping_db).await {
                Ok(_) => tracing::info!("keep-alive ping ok"),
                Err(e) => tracing::warn!("keep-alive ping failed: {e}"),
            }
        }
    });

    let state = AppState { db };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/products", get(list_products).post(create_product))
        .route("/products/:id", axum::routing::delete(archive_product))
        .route("/products/:id/variations", get(list_variations))
        .route("/variations/:id/stock", patch(update_stock))
        .route("/variations/:id/synced", post(mark_synced))
        .route("/queue", get(needs_update_queue))
        .with_state(state)
        .layer(cors);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

type ApiResult<T> = Result<T, (StatusCode, String)>;

fn db_err(e: sqlx::Error) -> (StatusCode, String) {
    tracing::error!("db error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, format!("database error: {e}"))
}

const VALID_STOCK: [&str; 4] = ["In Stock", "Out of Stock", "Discontinued", "Unknown"];

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

fn product_from_row(r: &sqlx::postgres::PgRow) -> ProductRow {
    ProductRow {
        id: r.get("id"),
        parent_sku: r.get("parent_sku"),
        name: r.get("name"),
        category: r.get("category"),
        platform: r.get("platform"),
        url: r.get("url"),
        image_url: r.get("image_url"),
        short_description: r.get("short_description"),
        notes: r.get("notes"),
        variation_count: r.get("variation_count"),
        in_stock_count: r.get("in_stock_count"),
        rollup_stock: r.get("rollup_stock"),
    }
}

fn variation_from_row(r: &sqlx::postgres::PgRow) -> VariationRow {
    VariationRow {
        id: r.get("id"),
        product_id: r.get("product_id"),
        variation_sku: r.get("variation_sku"),
        variation_name: r.get("variation_name"),
        price: r.get("price"),
        qty: r.get("qty"),
        stock: r.get("stock"),
        web_status: r.get("web_status"),
        sync: r.get("sync"),
        last_stock_change: r.get("last_stock_change"),
    }
}

async fn list_products(State(s): State<AppState>) -> ApiResult<Json<Vec<ProductRow>>> {
    let rows = sqlx::query(
        r#"SELECT id, parent_sku, name, category::text AS category, platform,
                  url, image_url, short_description, notes,
                  variation_count, in_stock_count, rollup_stock
           FROM product_rollup ORDER BY name"#,
    )
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;
    Ok(Json(rows.iter().map(product_from_row).collect()))
}

async fn list_variations(
    State(s): State<AppState>,
    Path(product_id): Path<i64>,
) -> ApiResult<Json<Vec<VariationRow>>> {
    let rows = sqlx::query(
        r#"SELECT id, product_id, variation_sku, variation_name, price, qty,
                  stock::text AS stock, web_status::text AS web_status,
                  sync::text AS sync, last_stock_change
           FROM variations WHERE product_id = $1 ORDER BY variation_name"#,
    )
    .bind(product_id)
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;
    Ok(Json(rows.iter().map(variation_from_row).collect()))
}

async fn update_stock(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateStock>,
) -> ApiResult<Json<VariationRow>> {
    if !VALID_STOCK.contains(&body.stock.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("invalid stock value {:?}; allowed: {:?}", body.stock, VALID_STOCK),
        ));
    }
    let row = sqlx::query(
        r#"UPDATE variations SET stock = $1::stock_status WHERE id = $2
           RETURNING id, product_id, variation_sku, variation_name, price, qty,
                     stock::text AS stock, web_status::text AS web_status,
                     sync::text AS sync, last_stock_change"#,
    )
    .bind(&body.stock)
    .bind(id)
    .fetch_optional(&s.db)
    .await
    .map_err(db_err)?
    .ok_or((StatusCode::NOT_FOUND, "variation not found".to_string()))?;
    Ok(Json(variation_from_row(&row)))
}

async fn mark_synced(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResult<impl IntoResponse> {
    let n = sqlx::query("UPDATE variations SET sync = 'Synced' WHERE id = $1")
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err((StatusCode::NOT_FOUND, "variation not found".to_string()));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn needs_update_queue(State(s): State<AppState>) -> ApiResult<Json<Vec<QueueRow>>> {
    let rows = sqlx::query(
        r#"SELECT v.id AS variation_id, p.name AS product_name,
                  v.variation_name, v.variation_sku,
                  v.stock::text AS stock, v.web_status::text AS web_status, p.url
           FROM variations v JOIN products p ON p.id = v.product_id
           WHERE v.sync = 'Needs update' AND p.archived_at IS NULL
           ORDER BY p.name"#,
    )
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;
    Ok(Json(
        rows.iter()
            .map(|r| QueueRow {
                variation_id: r.get("variation_id"),
                product_name: r.get("product_name"),
                variation_name: r.get("variation_name"),
                variation_sku: r.get("variation_sku"),
                stock: r.get("stock"),
                web_status: r.get("web_status"),
                url: r.get("url"),
            })
            .collect(),
    ))
}

async fn create_product(
    State(s): State<AppState>,
    Json(p): Json<NewProduct>,
) -> ApiResult<impl IntoResponse> {
    let mut tx = s.db.begin().await.map_err(db_err)?;
    let category = p.category.unwrap_or_else(|| "Tea".to_string());
    let product_id: i64 = sqlx::query_scalar(
        r#"INSERT INTO products (parent_sku, name, category, url, image_url, short_description)
           VALUES ($1, $2, $3::product_category, $4, $5, $6) RETURNING id"#,
    )
    .bind(&p.parent_sku)
    .bind(&p.name)
    .bind(&category)
    .bind(&p.url)
    .bind(&p.image_url)
    .bind(&p.short_description)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_err)?;

    for v in &p.variations {
        sqlx::query(
            r#"INSERT INTO variations (product_id, variation_sku, variation_name, price, qty, stock)
               VALUES ($1, $2, $3, $4, $5, $6::stock_status)"#,
        )
        .bind(product_id)
        .bind(&v.variation_sku)
        .bind(v.variation_name.clone().unwrap_or_else(|| "Standard".to_string()))
        .bind(v.price)
        .bind(v.qty)
        .bind(v.stock.clone().unwrap_or_else(|| "In Stock".to_string()))
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": product_id }))))
}

async fn archive_product(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResult<impl IntoResponse> {
    let n = sqlx::query("UPDATE products SET archived_at = now() WHERE id = $1 AND archived_at IS NULL")
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(db_err)?
        .rows_affected();
    if n == 0 {
        return Err((StatusCode::NOT_FOUND, "product not found or already archived".to_string()));
    }
    Ok(Json(serde_json::json!({ "archived": true })))
}
