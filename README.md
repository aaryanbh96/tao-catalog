# Tao of Tea Catalog & Inventory System

A cloud-backed inventory system for a specialty tea manufacturer's ~280-product catalog —
replacing a fragile shared spreadsheet with a Postgres database, a Rust API, and a native
desktop app distributed to non-technical staff.

**Stack:** Rust (axum) · PostgreSQL (Supabase) · Tauri · Docker · Render · Python (ETL)
**Write-up:** [rearyan.com](https://rearyan.com) · **Built by:** [Aryan Bhardwaj](https://github.com/aaryanbh96)

---

## The problem

The Tao of Tea sells a few hundred products, most in several sizes and packagings. Stock
isn't tracked per product — it's per *variation*: the classic tin can be in stock while
the bulk bag is sold out. The previous approach was a shared spreadsheet, which fails
quietly and badly: Excel coerces SKUs into numbers and drops leading zeros, concurrent
edits corrupt the file, and nothing reliably signals that the website needs updating when
something sells out.

The brief was robust, low-maintenance, and usable by people who never open a terminal.

## Architecture

```
  Desktop app (Tauri)          Cloud service (Render)         Database (Supabase)
  one per machine     ───────▶ Rust + axum            ──────▶ PostgreSQL
  thin UI                      the only writer                 source of truth
```

- **Database — PostgreSQL on Supabase.** Holds all data; integrity is enforced here
  (typed columns, enums, triggers, soft-deletes) so nothing can corrupt it regardless of
  what connects.
- **Service — Rust (axum + sqlx) on Render.** The single gatekeeper between the apps and
  the database. Runs a keep-alive ping so the free tier never pauses.
- **App — Tauri desktop app.** A web UI wrapped in a native window, installed per machine,
  talking to the cloud service. No data lives on the machine.

Cloud-hosted rather than tied to an office computer, so there's no single point of failure:
if any machine goes down, the data is safe and the rest keep working.

## Design decisions

| Decision | Why |
|----------|-----|
| Database over shared spreadsheet | One type per column — SKUs stay text (leading zeros survive), stock values come from a fixed set. Correctness is structural. |
| A service in front of the DB | Integrity and sync logic live in one place, not copied across app installs. |
| Cloud over an always-on office PC | Removes the single point of failure the brief explicitly ruled out. |
| Workflow enforced by a DB trigger | Stock changes auto-queue the website update — the queue can't be bypassed or forgotten. |

## Data model

Two tiers: **products** (parent) → **variations** (the sellable units, where stock lives).
A product's stock rolls up: in stock if all variations are, out if all are, *partial* if
mixed. The pipeline recovers per-variation SKU/price/stock from a raw web scrape,
normalizes into this model, and validates every row on import — reading SKUs as text so
the leading-zero corruption can't return. Imports are all-or-nothing in a single
transaction.

**281 products · 381 tracked variations · 0 corrupted SKUs.**

## What it does

- **Catalog view** — products with rolled-up stock badges (in stock / out / partial).
- **One-tap stock changes** — a dropdown per variation; saving instantly queues the
  website update via a database trigger.
- **Needs-update queue** — a live list of items marked out of stock but not yet changed on
  the website, cleared with "mark done."
- **Add & archive** — guided forms with validation; deletes are soft, so nothing is lost.
- **Distributed** — a native installer per machine, reaching the cloud from anywhere.

## Repository layout

```
service/          Rust service (axum + sqlx)
  src/            main.rs (endpoints), models.rs
  migrations/     0001_init.sql — schema: tables, enums, triggers, rollup view
  Dockerfile      how Render builds it
app/
  dist/           index.html — the entire UI
  src-tauri/      Tauri desktop wrapper + icons
scripts/
  import_catalog.py  validated Excel → Postgres import (keeps SKUs as text)
  test_service.py    endpoint smoke tests
docs/             technical reference + staff guide + deploy guides
render.yaml       deploy config (build from repo)
```

## Running it

In short:

**Database:** apply `service/migrations/0001_init.sql` to a Postgres instance, then
`python scripts/import_catalog.py catalog.xlsx "<connection-string>"`.

**Service (local):** `cd service && cargo run` (needs a `.env` with `DATABASE_URL`).

**Service (deploy):** push to a repo connected to Render; it builds from the Dockerfile.
Use Supabase's **connection-pooler** string for `DATABASE_URL` (the direct host is
IPv6-only and unreachable from Render).

**App:** `cd app && npm install && npm run tauri dev` to run, `npm run tauri build` to
produce installers.

## Endpoints

`GET /products` · `POST /products` · `DELETE /products/:id` (archive) ·
`GET /products/:id/variations` · `PATCH /variations/:id/stock` ·
`POST /variations/:id/synced` · `GET /queue` · `GET /health`

## Notable problems solved

Recovering variation-level data from a scrape that only captured product-level HTML; a
silent UI render bug that dropped a second variation; Windows/Excel SKU type-coercion; a
Rust dependency-version clash in the Tauri build; and a deployment issue where the cloud
service couldn't reach the database over IPv6 — fixed by switching to the connection
pooler. Each was diagnosed from the actual error, not worked around.

## Possible extensions

The schema carries a `platform` field for tracking the same products across other sales
channels, and the sync queue is the natural hook for a WooCommerce write-back that updates
the website automatically.

---

*Built June 2026. Developed with AI-assisted tooling; the architecture, integration,
debugging, and deployment decisions are mine.*
