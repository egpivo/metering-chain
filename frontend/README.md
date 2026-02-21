# Phase 4 Frontend

Deterministic operations UI for settlement lifecycle, claims, payouts, disputes, and policy (see `.local/phase4_frontend_plan_design.md`).

## Run

```bash
cd frontend && npm install && npm run dev
```

Open http://localhost:5173. The app uses **mock data** by default (`MockAdapter`). To use the real CLI backend, run in an environment where the `metering-chain` binary and Node are available and swap the adapter (e.g. `createCliAdapter({ dataDir: '...' })` in `AdapterProvider`).

## Build

```bash
npm run build
```

Output is in `dist/`.

## Tests

```bash
npm run test
```

Runs Vitest once. Use `npm run test:watch` for watch mode. Integration tests for the Phase 4 demo page live in `src/pages/DemoPhase4Page.test.tsx` (slider recompute → table update, mismatch blocks resolve CTA, missing evidence shows EVIDENCE_NOT_FOUND).

## Demo (Phase 4 closing)

- **Route:** `/demo/phase4`
- **Mode:** Snapshot (loads `public/demo_data/phase4_snapshot.json`). No API key.
- **Flow:** Adjust date range, granularity, policy sliders (operator/protocol/reserve bps, dispute window) → **Recompute** → table updates. Select a window to see Integrity/Evidence panel; **Resolve Dispute** is enabled only when compare status = MATCH and window is Disputed.

### Demo backend proxy (Week 2, Day 6+7)

Server that serves the same demo windows shape over HTTP (for BYOK and future Dune):

**Dev setup (recommended):** Run the proxy and the app so `/api` is same-origin and no CORS:

1. Terminal 1: `npm run demo:server` (listens on http://localhost:3001).
2. Terminal 2: `npm run dev`. Vite proxies **/api** → http://localhost:3001 (`vite.config.ts`), so the app calls `fetch('/api/demo/windows')` and the request is forwarded. Do **not** set `VITE_DEMO_PROXY_URL` in this setup.

- **GET /api/demo/windows** with query params: `start_date`, `end_date`, `owner`, `service_id`, `window_granularity`, `operator_share_bps`, `protocol_fee_bps`, `reserve_bps`, `top_n`.
- Fixed allowlist only; max date span 365 days; row cap 500; request timeout 15s; in-memory cache TTL 5 min.
- Response includes **`_meta`**: `{ mode: 'snapshot', key_provided: boolean, key_used: false }` so callers know the key is accepted but not yet used (Dune not wired). Frontend ignores `_meta` and uses `windows` only.
- **CORS:** If you set `VITE_DEMO_PROXY_URL=http://localhost:3001` (cross-origin), the proxy sends `Access-Control-Allow-Origin` for allowed dev origins (e.g. http://localhost:5173) and allows the `X-Dune-Api-Key` header.

**BYOK (Day 8, feature-flag):** Set `VITE_DEMO_BYOK_ENABLED=true`. The demo page shows "Dataset mode: Snapshot | Use my Dune key". In BYOK mode the key is session-only (masked input), sent to the proxy in `X-Dune-Api-Key` only; never stored. Leave `VITE_DEMO_PROXY_URL` unset when using the Vite proxy above.

## Structure

- `src/app/` — routing and layout
- `src/pages/` — Settlements, Settlement detail, Claims, Disputes, Policy, **DemoPhase4Page**
- `src/components/` — DataTable, StatusBadge, Timeline, ErrorBanner, ActionPanel, EvidenceCompareCard, **DemoControlPanel**, **CompareStatusChip**
- `src/domain/` — types, status labels, error mapping, **demo types (DemoWindowAggregate, DemoEvidenceView, DemoUiState)**
- `src/adapters/` — `FrontendDataAdapter`, `DemoAnalyticsAdapter`, **DemoSnapshotAdapter**, **createDemoProxyAdapter**, contexts
- `server/` — **demo-proxy.mjs** (Day 6+7: GET /api/demo/windows, cache, validation, limits)
