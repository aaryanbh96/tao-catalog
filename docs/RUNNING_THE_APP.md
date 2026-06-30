# Running the app

The app is the desktop window your staff use. It's a web UI that talks to the service's
API. You can run it two ways: as a plain web page (fastest, no extra tooling) or wrapped
as a real desktop app with Tauri.

> The service must be running first (`cd service && cargo run`). The app talks to it at
> `http://localhost:8080`. If the service is on a different machine, change the `API`
> constant near the top of the `<script>` in `app/index.html` to that machine's address,
> e.g. `http://192.168.1.50:8080`.

## Option 1 — run as a web page (quickest)

From the `app` folder:
```
python -m http.server 9090
```
Then open `http://localhost:9090` in a browser. That's it. The app loads your catalog,
lets you toggle stock, work the queue, add and archive products. Good for trying it and
for the machine that also runs the service.

Why not just double-click `index.html`? Browsers block a `file://` page from calling
`localhost`, so it must be *served* (the command above) or wrapped in Tauri.

## Option 2 — build the desktop app with Tauri

This produces a real installable window with an icon. One-time tooling:

1. Install the Tauri CLI:
   ```
   cd app
   npm install
   ```
   (Needs Node.js. Tauri also needs the system WebView — on Windows that's "WebView2",
   which is already on Windows 10/11. On first `npm install`/build Tauri tells you if
   anything's missing.)

2. Run it in dev mode (opens the window, hot-reloads):
   ```
   npm run tauri dev
   ```

3. Build the installer to hand to the other machines:
   ```
   npm run tauri build
   ```
   The installer lands under `app/src-tauri/target/release/bundle/`.

## What the app does

* **Catalog tab** — products with rolled-up stock (green / red / amber "Partial").
  Click a product to expand its variations; each has a stock dropdown that saves
  instantly. "Archive" soft-deletes (hides, doesn't destroy). "+ Add product" opens a
  form for a new product and its variations.
* **Needs update tab** — the website-sync queue. Anything marked out of stock shows here
  until someone updates the website and clicks "Mark done".
* **Light/Dark** toggle, search, and stock filter.

Every change goes through the service to the database — there's no local file to get out
of sync, and the rules (valid stock values, the auto-sync trigger, soft delete) are
enforced server-side.
