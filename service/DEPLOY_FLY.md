# Deploying the service to Fly.io (free, always-on, no office machine needed)

This puts the service in the cloud next to the database, so there's no dependency on
any office computer. The apps on all machines point at the Fly URL.

## One-time setup

1. **Install the Fly CLI** (`flyctl`). On Windows, in PowerShell:
   ```
   iwr https://fly.io/install.ps1 -useb | iex
   ```
   Then restart your terminal so `fly` is on the PATH.

2. **Sign up / log in** (free, no card needed for the free tier):
   ```
   fly auth signup        # or: fly auth login
   ```

## Deploy

From the `service` folder (the one with `fly.toml` and the `Dockerfile`):

1. **Create the app** (registers the name; doesn't deploy yet):
   ```
   fly launch --no-deploy
   ```
   - If it asks to copy/overwrite `fly.toml`, keep the existing one.
   - If the name `tao-catalog-service` is taken, it'll ask for another — pick anything
     unique and note it; that becomes part of your URL.
   - Pick a region near you when prompted (London = `lhr`, Frankfurt = `fra`, etc.).

2. **Give it the database connection string** as a secret (never goes in the code):
   ```
   fly secrets set DATABASE_URL="postgresql://postgres:YOURPASSWORD@db.rvfbtwbodhkftuydimwc.supabase.co:5432/postgres"
   ```
   Use your real Supabase string, in quotes, no brackets around the password.

3. **Deploy:**
   ```
   fly deploy
   ```
   First deploy builds the Docker image and takes a few minutes. When done, Fly prints
   your URL, like `https://tao-catalog-service.fly.dev`.

4. **Test it:**
   ```
   fly open /health
   ```
   or just visit `https://YOUR-APP.fly.dev/health` in a browser — you should see
   `{"status":"ok"}`.

## Point the app at it

In `app/dist/index.html`, change the API line near the top of the `<script>`:
```
const API = "https://YOUR-APP.fly.dev";
```
(no trailing slash). Then rebuild the desktop app:
```
cd app
npm run tauri build
```
That installer now works on **any** machine, in or out of the office — they all reach
the cloud service. No localhost, no host machine.

## Notes

- The free tier keeps one small machine running (we set `min_machines_running = 1` and
  `auto_stop_machines = false`), so there's no cold-start delay.
- The daily keep-alive ping inside the service keeps Supabase awake; Fly keeps the
  service awake. Nothing to babysit.
- To update the service later: change code, run `fly deploy` again.
- To see logs: `fly logs`. To check status: `fly status`.
