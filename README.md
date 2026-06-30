# Tao of Tea Catalog

Internal inventory system for the Tao of Tea catalog. Tracks products, their
variations, per-variation stock, and the website-sync workflow (production marks
something out of stock → it appears in a "needs update" queue → whoever owns the
website updates it and marks it done).

## Architecture

```
3 office machines ──(local network / internet)──> Rust service ──> Postgres (Supabase)
   Tauri app                                       (only writer)      source of truth
```

* **Database — Postgres (Supabase free tier).** Hosted, so there's no dependency on
  an always-on office computer. Standard Postgres, so it can move to any other host
  by changing one connection string. A daily keep-alive ping keeps the free tier from
  pausing.
* **Service — Rust (axum + sqlx).** The single gatekeeper. All reads and writes go
  through it; it enforces the rules so the apps can't put the data into a bad state.
* **Apps — Tauri.** Reuse the existing dashboard UI. Editing happens here through
  guided forms, never by hand-editing a spreadsheet (see "Why not Excel" below).

## Data model

Two tiers: a **product** (parent) has one or more **variations** (the sellable units).
Stock lives on the variation. A product's overall stock is *rolled up*: in stock only
if every variation is, out of stock if all are, otherwise "Partial".

Key design choices, all in `service/migrations/0001_init.sql`:

* **SKUs are TEXT, never numbers** — leading zeros like `09100` are preserved.
* **Controlled vocabularies are enums** — category, stock status, sync state, and
  website status can only hold valid values, so "Tea / tea / Teas" drift is impossible.
* **Soft delete** — `archived_at` hides rows instead of destroying them; they're
  restorable.
* **Stock-change trigger** — changing a variation's stock automatically stamps the
  time, sets sync to "Needs update", and updates website status. The write-back queue
  is enforced in the database, not just the app.

## Why not edit the spreadsheet directly

Excel silently reformats data: it turns SKUs into numbers (dropping leading zeros),
guesses types, and stores values that *look* identical but aren't. That class of bug
is invisible and bites any program reading the file. The database enforces one type
per column, so it can't happen. Excel is therefore **export-only** (backups, snapshots);
editing is done in the app. A validated Excel *import* exists for the initial load and
could be reused for bulk edits later, but it always reads SKUs as text and validates
every row first.

## First-time setup

1. Create a Supabase project; grab the Postgres connection string.
2. Apply the schema:
   ```
   psql "<connection string>" -f service/migrations/0001_init.sql
   ```
3. Load the cleaned catalog:
   ```
   pip install -r scripts/requirements.txt
   python scripts/import_catalog.py catalog.xlsx "<connection string>"
   ```
   The import runs in a single transaction — if any row fails validation, nothing is
   written. Warnings (e.g. a 4-digit SKU that may have lost a leading zero) are
   reported but don't block.

## Layout

```
service/
  migrations/0001_init.sql   schema (types, tables, triggers, rollup view)
  src/                       Rust service (added next)
scripts/
  import_catalog.py          one-time / bulk Excel import with validation
  requirements.txt
docs/
```

## Status

- [x] Schema, validated against real data
- [x] Excel import, validated against real data (381 rows → 281 products, 381 variations)
- [x] Rust service (axum + sqlx) with endpoints + daily keep-alive ping — see `docs/RUNNING_THE_SERVICE.md`
- [ ] Tauri app wiring
- [ ] WooCommerce REST write-back (later)

See `docs/RUNNING_THE_SERVICE.md` for how to build, run, and test the service.
