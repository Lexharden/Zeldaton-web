# HiveShock → Zeldatón ingestion contract

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
| `STATS_UPDATED` | `stats: { age?, hearts?, maxHearts?, rupees?, skulltulas?, bossesDefeated? }` | Merged into the racer. `age` is the Link being played: `"child"` or `"adult"` |
| `GAME_FINISHED` | – | Accepted **only if every required objective is completed**; the racer finishes and the first one to do so is the winner |
| `CHAT_EVENT` | `count?` | Counts toward the HiveShock chat metric |
| `STREAM_STATE` | `live`, `viewers?` | The racer is (or is no longer) broadcasting on TikTok/Twitch and how many people watch. Accepted at any time while connected (no game session needed). Shown on the streams page; cleared automatically if HiveShock disconnects |
| `TIME_DONATION` | `deltaSeconds`, `source: { platform, currency, amount, gift?, giftCount?, viewer? }` | A viewer donation changes the racer's time: positive adds, negative removes. **`id` is required.** See below. Reply: `TIME_APPLIED` |

Catalog ids (objectives, items) live in the database and are edited from the organizer panel; read them with
`GET /api/catalog` (enabled entries, each tagged `child` / `adult` / `both`). The factory catalog (~60 items, 10
objectives) is in `backend/src/catalog.rs`. Anything not enabled in the catalog is rejected with `invalid`, so HiveShock
downloads the catalog when it connects and only sends ids it lists. `GAME_FINISHED` needs the objectives in
`rules.requiredObjectiveIds` (`GET /api/event`), not necessarily the whole catalog.
Area ids are free-form; ids the website knows (e.g. `water-temple`) are translated, others are shown as-is.

### Time from donations (`TIME_DONATION`)

The **streamer** decides in HiveShock how many seconds each TikTok diamond or Twitch bit is worth and whether
donations add or remove time; HiveShock converts and sends the result. The **organizer** sets the limits in
`/admin → Evento → Tiempo por donaciones` (`donationTime` in `GET /api/event`):

| Field | Meaning (seconds) |
| --- | --- |
| `enabled`, `allowAdd`, `allowRemove` | Off, or a direction not allowed → `ERROR not_allowed` |
| `maxSecondsPerDonation` | One donation never changes more than this |
| `maxAddedSecondsPerDay` / `maxRemovedSecondsPerDay` | Per racer and day; reset with the daily budget |

```json
{ "type": "TIME_DONATION", "id": "don-3f2c…", "deltaSeconds": -90,
  "source": { "platform": "tiktok", "currency": "diamonds", "amount": 30, "gift": "Rose", "giftCount": 30, "viewer": "fan" } }
```

* `platform`/`currency`: `tiktok` + `diamonds` or `twitch` + `bits` (`amount` = the whole donation, a TikTok combo once).
* Only while the event is live, and not after the racer finished. Time can be added while offline or exhausted
  (an exhausted racer with time again goes back to online); removing the last second of a running game is the
  same as running out of time (`GAME_FORCE_CLOSE`).
* The clock stays between 0 and two daily budgets. Less than asked may be applied: the reply says why.
* The `id` is stored in the ledger (`time_donations`): a retry with the same id is acknowledged (`ACK`) and never
  applied twice, **even after a server restart**. Every donation is listed in `/admin → Donaciones`.

Reply:

```json
{ "type": "TIME_APPLIED", "id": "don-3f2c…", "requestedSeconds": -90, "appliedSeconds": -90,
  "limitedBy": "daily_limit", "clock": { "remainingMs": 13500000, "status": "live", "...": "..." } }
```

`limitedBy` (only when less was applied): `per_donation`, `daily_limit`, `clock_max`, `clock_zero`. When the
time really changes, the public site gets `CLOCK_SYNC` and a `LIVE_ACTIVITY` (`TIME_ADDED` / `TIME_REMOVED`,
without the viewer's name).

## Messages the server sends back

- `{"type":"ACK","id":"..."}`: applied (or already applied).
- `{"type":"CLOCK","clock":{ remainingMs, status, resetAtUtc, serverTimeUtc, racerId }}`: official time, in reply to `HELLO`/`HEARTBEAT`.
  It is also **pushed on its own** (no `id`) right after an organizer action or a status change that
  affects the racer's clock (pause/resume, adjust-time, reset-day, daily reset), so the overlay does not
  have to wait for the next heartbeat. Clients must accept a `CLOCK` at any time.
- `{"type":"TIME_APPLIED", ...}`: answer to `TIME_DONATION` (see above).
- `{"type":"ERROR","id":"...","code":"...","message":"..."}`. Codes: `invalid`, `event_not_live`,
  `out_of_sequence`, `requirements_not_met`, `exhausted`, `not_allowed`, `replaced`.
- `{"type":"GAME_FORCE_CLOSE"}`: **the daily time reached zero: close the game now.** Also sent if a
  session start is attempted with no time left.

## Rules the server enforces

- The clock counts down only while the racer is `live` (session started, not paused).
- The budget resets every day at `dailyResetLocalTime` (06:00) **in the racer's own IANA timezone**.
- No heartbeat for `STALE_HEARTBEAT_SECS` (default 20 s) → racer goes offline and the clock stops; it
  resumes from the official remaining time after the next `HELLO`.
- After a server restart nobody is connected; no time is counted for the downtime.
