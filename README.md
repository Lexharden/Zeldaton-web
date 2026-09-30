# Zeldathon

Live Ocarina of Time community race. Vue 3 website + Rust backend that keeps the official clocks,
progress and ranking, fed by HiveShock.

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
| Organizer `/api/admin` | Bearer `ADMIN_TOKEN`: event, racers + channels, tokens, pause/resume/force-close/reset-day/adjust-time/finish, audit log |

Every organizer action lands in the `audit_log` table, the official record of the session.

## Deploy (Docker + native nginx on the VPS)

Backend and website run as containers bound to localhost (`127.0.0.1:8080` / `:8081`); the nginx
installed on the VPS terminates TLS and proxies to them.

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

## Before the event

- Fill each racer's Twitch / TikTok handle, country and timezone (`PATCH /api/admin/racers/:id`).
- Confirm the start time (`PUT /api/admin/event`, default 2026-10-07 06:00 Mexico City) and end date.
- Give every racer their token; rotate any that leak.
