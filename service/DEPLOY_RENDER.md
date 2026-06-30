# Deploying the service to Render (free, no card, no office machine)

Render builds the service straight from your GitHub repo using the Dockerfile.
Free tier, no credit card. It sleeps after ~15 min idle (≈40s wake-up on the next
request); the uptime pinger at the end keeps it awake during the workday.

## Prerequisite: push the latest code to GitHub

Render reads from GitHub, so commit and push first (the `render.yaml`, `Dockerfile`,
and the `PORT` change all need to be in the repo). From the repo root:
```
git add .
git commit -m "Add Render + Docker deploy config"
git push
```

## Deploy on Render

1. Go to **render.com**, sign up / log in with your GitHub account (no card needed).
2. Click **New** → **Blueprint** (this reads the `render.yaml` in your repo).
   - Connect your `tao-catalog` repo when prompted.
   - Render detects `render.yaml` and proposes the `tao-catalog-service` web service.
3. Before it deploys, it'll ask for the value of **DATABASE_URL** (we marked it
   `sync: false` so it's entered as a secret, not stored in the repo). Paste your
   Supabase connection string (no brackets around the password).
4. Click **Apply** / **Create**. The first build runs the Docker image — a few minutes.
5. When it's live, Render gives you a URL like
   `https://tao-catalog-service.onrender.com`.

## Test it

Visit `https://YOUR-APP.onrender.com/health` — you should see `{"status":"ok"}`.
(The very first hit may take ~40s if it was asleep — that's the cold start.)

## Point the app at it

In `app/dist/index.html`, change the API line:
```
const API = "https://YOUR-APP.onrender.com";
```
(no trailing slash). Rebuild the desktop app:
```
cd app
npm run tauri build
```
That installer now works on any machine — they all reach the cloud service.

## Keep it awake during the workday (free, no card)

To avoid the cold-start delay, ping the service every few minutes so it doesn't sleep:

1. Go to **uptimerobot.com**, sign up free (no card).
2. Add a new monitor:
   - Type: **HTTP(s)**
   - URL: `https://YOUR-APP.onrender.com/health`
   - Interval: **5 minutes**
3. Save. UptimeRobot now hits `/health` every 5 minutes, so Render keeps the service
   running and there's effectively no wake-up delay. (It also emails you if the service
   ever goes down — a free bonus.)

## Notes

- Updating later: push to GitHub → Render auto-redeploys.
- Logs: in the Render dashboard, the service's **Logs** tab.
- The free instance has limited RAM; for 381 products it's plenty.
