# Zeldatón

Live Ocarina of Time community race. Vue 3 website + Rust backend that keeps the official clocks,
progress and ranking, fed by HiveShock. Developed by **Yafel GH**, powered by **HiveShock**.

```
HiveShock ──WS/POST──▶ backend (Rust, Axum, SQLite) ──REST + WS──▶ website (Vue 3)
```

## Website only (mock data, no backend)

```bash
npm install
npm run dev            # VITE_MOCK_MODE=true in .env: simulated race, no server needed
```

Preview states with `?state=upcoming|live|paused|finished`.

## Full stack locally

```bash
# 1. backend (first run creates the database and prints each racer's ingest token ONCE)
cd backend && cp .env.example .env    # set ADMIN_TOKEN, optionally DEV_TOKENS_FILE=dev-tokens.json
cargo run -p zeldathon-server

# 2. simulate HiveShock for the 9 racers (needs DEV_TOKENS_FILE; ADMIN_TOKEN flips the event live)
ADMIN_TOKEN=... DEV_TOKENS_FILE=dev-tokens.json SIM_SPEED=1 cargo run -p zeldathon-server --bin simulate

# 3. website against the real backend
VITE_MOCK_MODE=false VITE_API_URL=http://localhost:8080/api VITE_WS_URL=ws://localhost:8080/ws npm run dev
```

## Tests

```bash
cargo test            # rules, clock/DST, REST + WebSocket + admin + persistence (backend/tests)
npx vitest run        # frontend units + wire contract against ./contract
npm run build         # type-check + production build
```

`contract/` holds the JSON the server emits. Rust generates and verifies it
(`UPDATE_CONTRACT=1 cargo test --test contract` after an intentional change); the website test
validates the same files, so the two sides cannot drift apart silently.

## Backend API

| | |
| --- | --- |
| Public REST `/api` | `/event`, `/racers`, `/racers/:id`, `/standings`, `/streams`, `/activity`, `/hiveshock/stats`, `/clocks`, `/health` |
| Public WebSocket | `/ws`: `CLOCK_SNAPSHOT` on connect, then live events (types in `src/types/websocket.ts`) |
| HiveShock ingestion | `WS /ingest`, `POST /ingest/events`; see [docs/hiveshock-ingest.md](docs/hiveshock-ingest.md) |
| Public catalog | `GET /api/catalog` (items and objectives by Link, enabled only) + `CATALOG_UPDATED` on the socket |
| Organizer `/api/admin` | Session cookie (roles admin / moderator) or Bearer `ADMIN_TOKEN` (emergency): overview, event start/pause/resume/finish, racers + channels, tokens, pause/resume/force-close/reset-day/adjust-time/finish, catalog, accounts, audit log, time donations |

Racer photos: uploaded in the panel (`POST /api/admin/racers/:id/photo`), kept in `UPLOADS_DIR/racers` (the data volume in
Docker) and served publicly at `/api/media/racers/<file>`.

Every organizer action lands in the `audit_log` table, the official record of the session (with who did it).

## Organizer panel (`/admin`)

A CMS inside the same website: sign in at `/admin` with an organizer account.

```bash
# backend/.env (first start only: creates the first admin when there are no users yet)
ADMIN_USER=yafel
ADMIN_PASSWORD=<at least 12 characters>      # empty = a random one is printed ONCE on start
```

| Page | What for |
| --- | --- |
| Panel | Live dashboard: **start / pause / finish the event**, who is connected, clocks, progress, alerts, activity |
| Event | Start/end, daily time, reset hour, win condition, objectives required to finish, **limits for time from donations** |
| Racers | Create/edit, **upload their photo** (shrunk in the browser; replaces the previous one), channels, **token** (shown once, rotate), control: pause, close game, adjust time (with reason), reset day, finish |
| Catalog | Items and objectives by **Child / Adult / Both Link** (~60 factory items): create, edit, hide, reorder. An item's **icon is just a file name** (`Hookshot-Art.png`) of a picture in `public/art/items`; the site loads it from that folder (a new picture needs a rebuild). The site and HiveShock update on their own |
| Donations | Time that TikTok gifts / Twitch bits added or removed (the **rate per diamond / bit is set in Event → Time from donations** and computed by the server, default 3 s per diamond; the table shows what HiveShock asked when it differs), per racer (today and whole event), the **top donors** (who moved the clock most; shown publicly on the home page, you can hide one or switch the board off) and one by one |
| Audit | Who did what and when |
| Accounts | Organizer accounts and roles (admin runs everything; moderator runs the race day) |

Security: Argon2id passwords, HttpOnly + SameSite=Strict session cookie (`Secure` automatic; force with
`COOKIE_SECURE`), CSRF token on every write, 5 wrong logins lock a username for 15 minutes, and every session
ends when a password changes or an account is disabled. `ADMIN_TOKEN` (Bearer) still works as an emergency key.

In development the panel talks to the backend through the Vite proxy (`/api`, `/ws`, `/ingest` → `127.0.0.1:8080`,
change it with `VITE_PROXY_TARGET`), so `npm run dev` + `cargo run -p zeldathon-server` is all you need.

## Deploy (Docker + native nginx on the VPS)

Backend and website run as containers bound to localhost (`127.0.0.1:8080` / `:8081`; change the ports with
`BACKEND_PORT` / `FRONTEND_PORT` in `.env`); the nginx installed on the VPS terminates TLS and proxies to them.

```bash
cp .env.example .env            # set SITE_ADDRESS and ADMIN_TOKEN (openssl rand -hex 32)
docker compose up -d --build
docker compose logs backend     # first run prints the racer ingest tokens ONCE

sudo cp deploy/nginx.conf /etc/nginx/sites-available/zeldathon   # edit the domain
sudo ln -s /etc/nginx/sites-available/zeldathon /etc/nginx/sites-enabled/
sudo certbot certonly --nginx -d zeldathon.example.com           # or use --nginx on a plain :80 block first
sudo nginx -t && sudo systemctl reload nginx
```

The SQLite database lives in the `zeldathon_data` volume. Back it up with
`docker compose exec backend ...` or `docker run --rm -v zeldaton-web_zeldathon_data:/data -v $PWD:/out debian cp /data/zeldathon.db /out/`.

## Google Analytics 4 (optional)

Put your measurement id in the server `.env` and rebuild; that is all:

```bash
GA_MEASUREMENT_ID=G-XXXXXXXXXX      # .env next to docker-compose.yml
docker compose up -d --build frontend
```

(`npm run dev/build` reads `VITE_GA_MEASUREMENT_ID` instead.) Without an id nothing is loaded and no banner appears.
With one, visitors get a cookie notice with equal **Accept / Reject** buttons: nothing is requested from Google
until they accept (Consent Mode v2, script injected only after consent), rejecting later deletes the `_ga` cookies,
Global Privacy Control counts as a rejection, and `/admin` is never tracked. Page views are sent on every SPA
navigation. The policy lives at `/privacy` (and terms at `/terms`); "Cookie settings" in the footer reopens the notice.

### How the standings are decided

Finished racers first, in the order they crossed the line (the first one is the winner), then by less time really played.
Racing: more **required** objectives completed → higher progress (tenths of a percent) → more items → reached that number
of objectives earlier → less time really played → id. Donations and time adjustments never move anybody. The order lives in
`backend/src/standings.rs` and `src/utils/standings.ts`, and `contract/standings-cases.json` is read by both test suites so
they cannot drift apart. In `/admin → Panel` hover a racer's `#position` to see the values being compared.

### Discord notifications

Two channels, each with its own webhook (no bot needed). Switch each one on, and choose which notices it gets, in
`/admin → Evento → Discord`; the same card has a test button per channel.

| Channel | Notices |
| --- | --- |
| **Community** (`DISCORD_WEBHOOK_URL`) | 🔴 a racer goes live (with links to their page and TikTok / Twitch) · 🏆 someone finishes (the first is the winner) · ⏱️ a racer runs out of time · 👹 a boss is defeated · 🥇 a new leader |
| **Referees** (`DISCORD_STAFF_WEBHOOK_URL`) | 🔌 a long disconnection (and the return) · ⚠️ low time · 🚧 a donation cap is reached · 🚩 suspicious progress jump · 🔔 event started / paused / resumed / finished / reset · 🛠️ an organizer closed a game, adjusted time (who and why), reset a day or finished a racer |

Setup: Discord → channel settings → Integrations → Webhooks → New webhook → Copy URL → put it in the server's `.env`
(secrets: never share them) and rebuild. Optional `DISCORD_STAFF_ROLE_ID` makes critical referee alerts (disconnection, low time,
suspicious jump) mention that role; nothing else mentions anybody. Thresholds (minutes disconnected, minutes left, size and
speed of a "jump") are set in the same card.

How it behaves: the community channel only hears a **running, real** event (never rehearsal or before the start); the referees'
channel also works in rehearsal (tagged `[ENSAYO]`) so you can test it. One-off notices (boss, finish, ran out of time,
donation cap) are sent once even if the server restarts, and are forgotten when you reset the event. Repeats are damped
(live: 30 s steady and 30 min apart; leader: 1 min steady and 5 min apart). Each channel posts in order under Discord's rate
limit, most important first; the webhook URLs never appear in the panel or the logs.

Adding a new notice later: one variant in `backend/src/notify/mod.rs` (`Kind` and `Detail`), the place that emits it and one arm in
`notify/render.rs`; the panel lists kinds from the API.

### Rehearsal, then the real event

Racers can test with their real HiveShock first. In `/admin → Evento → Ensayo y reinicio`:

1. Turn on **Modo ensayo** and save: the public site shows a "test data" notice.
2. Start the event and let the racers play, donate, break things.
3. When they are ready, **Reiniciar evento**: type `REINICIAR`, pick the real start time. It wipes progress,
   items, clocks, donations, donors, activity and the winner, and puts the event back to "upcoming" (and
   ends rehearsal). **Racers, their HiveShock tokens, the catalog, pictures, accounts and the audit log stay**,
   so nobody reconfigures anything. Open browsers reload by themselves.

A real event that is live can never be reset (only a rehearsal can). Rehearsal cannot be turned on while the event
is live: pause first. API: `POST /api/admin/event/reset` `{ confirm, startAtUtc, leaveRehearsal }`, `PUT /api/admin/event`
`{ rehearsal }`; the socket sends `EVENT_UPDATED` on both.

## Before the event

- Fill each racer's Twitch / TikTok handle, country and timezone (`PATCH /api/admin/racers/:id`).
- Confirm the start time (`PUT /api/admin/event`, default 2026-10-07 06:00 Mexico City) and end date.
- Give every racer their token; rotate any that leak.
