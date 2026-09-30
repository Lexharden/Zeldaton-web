# HiveShock → Zeldathon ingestion contract

HiveShock (running on each streamer's machine) pushes game telemetry to the backend. The backend is
the **only authority** for the daily clock, ranking and winner: HiveShock reports facts, the server
decides what they mean.

## Connection

| Channel | URL | Use |
| --- | --- | --- |
| WebSocket (main) | `wss://<host>/ingest` | Continuous session, bidirectional (recommended) |
| HTTP (fallback) | `POST https://<host>/ingest/events` `{ "events": [ ... ] }` (≤ 100) | Clients that cannot hold a socket |

Authenticate with the racer's own token: `Authorization: Bearer <token>`. Tokens are generated when a
racer is created (printed once on first run, or returned by `POST /api/admin/racers` /
`POST /api/admin/racers/:id/token`). Only a SHA-256 hash is stored. Rotating a token drops any
connection using the old one. One connection per racer: a new one replaces the previous.

## Messages HiveShock sends

Every message is JSON with a `type`. Add an optional `"id"` (any unique string): a repeated id is
acknowledged but applied only once, so retries are safe.

| `type` | Fields | Effect |
| --- | --- | --- |
| `HELLO` | `clientVersion?` | Marks the racer online, starts the heartbeat window. Reply: `CLOCK` |
| `HEARTBEAT` | `gameRunning?` | Keeps the racer online. Send every ≤ 10 s. Reply: `CLOCK` |
| `SESSION_STARTED` / `SESSION_RESUMED` | – | Game is being played: the daily clock **starts counting**. Only accepted while the event is live |
| `SESSION_PAUSED` | – | Clock stops |
| `SESSION_ENDED` | – | Game closed by the player; racer goes back to online |
| `GAME_PROGRESS` | `progress: { percentage?, currentArea?, currentObjective?, completedObjectives? }` | Partial update. Objective ids must be in the catalog |
| `ITEM_ACQUIRED` | `item` | Must be a known item id. Repeats are ignored |
| `AREA_CHANGED` | `area` (≤ 64 chars) | Current area id |
| `BOSS_DEFEATED` | `boss` | Increments bosses defeated |
| `STATS_UPDATED` | `stats: { hearts?, maxHearts?, rupees?, skulltulas?, bossesDefeated? }` | Merged into the racer |
| `GAME_FINISHED` | – | Accepted **only if every required objective is completed**; the racer finishes and the first one to do so is the winner |
| `CHAT_EVENT` | `count?` | Counts toward the HiveShock chat metric |

Catalog ids (objectives, items) are in `backend/src/catalog.rs` and mirror `src/config/event.ts`.
Area ids are free-form; ids the website knows (e.g. `water-temple`) are translated, others are shown as-is.

## Messages the server sends back

- `{"type":"ACK","id":"..."}`: applied (or already applied).
- `{"type":"CLOCK","clock":{ remainingMs, status, resetAtUtc, serverTimeUtc, racerId }}`: official time, in reply to `HELLO`/`HEARTBEAT`.
- `{"type":"ERROR","id":"...","code":"...","message":"..."}`. Codes: `invalid`, `event_not_live`,
  `out_of_sequence`, `requirements_not_met`, `exhausted`, `replaced`.
- `{"type":"GAME_FORCE_CLOSE"}`: **the daily time reached zero: close the game now.** Also sent if a
  session start is attempted with no time left.

## Rules the server enforces

- The clock counts down only while the racer is `live` (session started, not paused).
- The budget resets every day at `dailyResetLocalTime` (06:00) **in the racer's own IANA timezone**.
- No heartbeat for `STALE_HEARTBEAT_SECS` (default 20 s) → racer goes offline and the clock stops; it
  resumes from the official remaining time after the next `HELLO`.
- After a server restart nobody is connected; no time is counted for the downtime.
