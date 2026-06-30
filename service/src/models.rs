use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

/// A product (parent) with its rolled-up stock, as returned to the apps.
#[derive(Debug, Serialize)]
pub struct ProductRow {
    pub id: i64,
    pub parent_sku: String,
    pub name: String,
    pub category: String,
    pub platform: String,
    pub url: Option<String>,
    pub image_url: Option<String>,
    pub short_description: Option<String>,
    pub notes: Option<String>,
    pub variation_count: i64,
    pub in_stock_count: i64,
    pub rollup_stock: String,
}

/// A single sellable unit.
#[derive(Debug, Serialize)]
pub struct VariationRow {
    pub id: i64,
    pub product_id: i64,
    pub variation_sku: String,
    pub variation_name: String,
    pub price: Option<Decimal>,
    pub qty: Option<i32>,
    pub stock: String,
    pub web_status: String,
    pub sync: String,
    pub last_stock_change: Option<DateTime<Utc>>,
}

/// One row in the "needs update" queue (variation joined to its product).
#[derive(Debug, Serialize)]
pub struct QueueRow {
    pub variation_id: i64,
    pub product_name: String,
    pub variation_name: String,
    pub variation_sku: String,
    pub stock: String,
    pub web_status: String,
    pub url: Option<String>,
}

// ---------- request bodies ----------

#[derive(Debug, Deserialize)]
pub struct UpdateStock {
    pub stock: String, // 'In Stock' | 'Out of Stock' | 'Discontinued' | 'Unknown'
}

#[derive(Debug, Deserialize)]
pub struct NewProduct {
    pub parent_sku: String,
    pub name: String,
    pub category: Option<String>,
    pub url: Option<String>,
    pub image_url: Option<String>,
    pub short_description: Option<String>,
    pub variations: Vec<NewVariation>,
}

#[derive(Debug, Deserialize)]
pub struct NewVariation {
    pub variation_sku: String,
    pub variation_name: Option<String>,
    pub price: Option<Decimal>,
    pub qty: Option<i32>,
    pub stock: Option<String>,
}
