-- Tao of Tea Catalog — initial schema
-- Design notes:
--   * SKUs are TEXT, never numeric, so leading zeros (e.g. '09100') survive.
--   * Two-tier model: a product has one or more variations. Stock lives on the variation.
--   * Soft delete via archived_at: rows are hidden, never destroyed, and restorable.
--   * Controlled vocabularies (stock status, sync state, category) are enforced by enums
--     so the "Tea/tea/Teas" drift and invalid statuses are impossible.

-- ---------- Enums (the fixed dropdowns) ----------
CREATE TYPE stock_status   AS ENUM ('In Stock', 'Out of Stock', 'Discontinued', 'Unknown');
CREATE TYPE sync_state     AS ENUM ('Synced', 'Needs update', 'Done');
CREATE TYPE website_status AS ENUM ('Live', 'Hidden', 'OOS');
CREATE TYPE product_category AS ENUM ('Tea', 'Teaware', 'Accessories', 'Gift Sets', 'Other');

-- ---------- Products (the parent) ----------
CREATE TABLE products (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    parent_sku      TEXT NOT NULL UNIQUE,           -- groups variations; TEXT on purpose
    name            TEXT NOT NULL,
    category        product_category NOT NULL DEFAULT 'Tea',
    platform        TEXT NOT NULL DEFAULT 'Tao of Tea (own site)',
    url             TEXT,
    image_url       TEXT,
    short_description TEXT,
    notes           TEXT,
    archived_at     TIMESTAMPTZ,                    -- NULL = active; set = soft-deleted
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------- Variations (the sellable units) ----------
CREATE TABLE variations (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    product_id      BIGINT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    variation_sku   TEXT NOT NULL UNIQUE,           -- TEXT; unique across the whole catalog
    variation_name  TEXT NOT NULL DEFAULT 'Standard',
    price           NUMERIC(10,2),                  -- money, not float; NULL allowed (unpriced)
    qty             INTEGER,
    stock           stock_status   NOT NULL DEFAULT 'In Stock',
    web_status      website_status NOT NULL DEFAULT 'Live',
    sync            sync_state     NOT NULL DEFAULT 'Synced',
    last_stock_change TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_variations_product  ON variations(product_id);
CREATE INDEX idx_variations_sync     ON variations(sync) WHERE sync = 'Needs update';
CREATE INDEX idx_variations_stock    ON variations(stock);
CREATE INDEX idx_products_active     ON products(archived_at) WHERE archived_at IS NULL;

-- ---------- keep updated_at honest ----------
CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_products_touch   BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
CREATE TRIGGER trg_variations_touch BEFORE UPDATE ON variations
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

-- When stock changes, stamp the change time and mark it as needing a website update.
CREATE OR REPLACE FUNCTION on_stock_change() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.stock IS DISTINCT FROM OLD.stock THEN
        NEW.last_stock_change = now();
        NEW.sync = 'Needs update';
        NEW.web_status = CASE WHEN NEW.stock = 'In Stock' THEN 'Live' ELSE 'OOS' END;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_stock_change BEFORE UPDATE ON variations
    FOR EACH ROW EXECUTE FUNCTION on_stock_change();

-- ---------- a convenient view: product with rolled-up stock ----------
CREATE VIEW product_rollup AS
SELECT p.*,
    COUNT(v.id)                                            AS variation_count,
    COUNT(v.id) FILTER (WHERE v.stock = 'In Stock')        AS in_stock_count,
    CASE
        WHEN COUNT(v.id) = 0 THEN 'Unknown'
        WHEN COUNT(v.id) FILTER (WHERE v.stock = 'In Stock') = COUNT(v.id) THEN 'In Stock'
        WHEN COUNT(v.id) FILTER (WHERE v.stock = 'In Stock') = 0 THEN 'Out of Stock'
        ELSE 'Partial'
    END                                                    AS rollup_stock
FROM products p
LEFT JOIN variations v ON v.product_id = p.id
WHERE p.archived_at IS NULL
GROUP BY p.id;
