# Running the service

The service is a small Rust web server that sits between the apps and the database.
It's the only thing that writes to Postgres, and it runs the daily keep-alive ping.

## One-time setup

1. Put your connection string in a `.env` file next to the service:
   ```
   cd service
   copy .env.example .env        (Windows)   |   cp .env.example .env   (Mac/Linux)
   ```
   Edit `.env` and set `DATABASE_URL` to your real Supabase string (no brackets around
   the password). Use the DIRECT connection on port 5432, not the pooler.

   > `.env` is gitignored — it never gets committed.

## Build and run (development)

```
cd service
cargo run
```

First build downloads dependencies and takes a few minutes; later builds are fast.
When it's up you'll see:
```
connected to database
listening on http://0.0.0.0:8080
```

## Test it

In a second terminal, with the service running:
```
python scripts/test_service.py
```
It hits every endpoint, exercises the stock-change trigger and the sync queue, and
reverts anything it changed. All lines should say PASS.

## Build for production (the always-on host)

```
cd service
cargo build --release
```
The binary lands at `service/target/release/tao-catalog-service` (`.exe` on Windows).
Copy it plus a `.env` to the host machine and run it. Set it to start on boot and keep
the machine awake (Windows: Task Scheduler "At startup" + power settings "never sleep").

## Endpoints

| Method | Path                          | What it does                                  |
|--------|-------------------------------|-----------------------------------------------|
| GET    | `/health`                     | liveness check                                |
| GET    | `/products`                   | all products with rolled-up stock             |
| POST   | `/products`                   | create a product + its variations             |
| DELETE | `/products/:id`               | soft-delete (archive) a product               |
| GET    | `/products/:id/variations`    | variations of one product                     |
| PATCH  | `/variations/:id/stock`       | change stock (trigger updates sync/web_status)|
| POST   | `/variations/:id/synced`      | mark a variation back to Synced               |
| GET    | `/queue`                      | the "needs update" queue                      |

All responses are JSON. The apps talk to this; nothing talks to Postgres directly.
