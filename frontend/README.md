# Phase 4 Frontend

Settlement lifecycle UI (claims, payouts, disputes, policy). See `.local/phase4_frontend_plan_design.md` for design.

## Run

```bash
cd frontend && npm install && npm run dev
```

Open http://localhost:5173. Default data: snapshot from `public/demo_data/phase4_snapshot.json`. `VITE_USE_MOCK_ADAPTER=true` forces the mock adapter.

## Build & deploy

- **Build:** `npm run build` → output in `dist/`.
- **GitHub Pages:** On push to `main`, `.github/workflows/deploy-pages.yml` builds with `BASE_PATH=/metering-chain/` and deploys. Enable in **Settings → Pages → Source: GitHub Actions**. Site: `https://<org>.github.io/metering-chain/`.

## Tests

```bash
npm run test
```

Vitest; `npm run test:watch` for watch. Demo integration tests in `src/pages/DemoPhase4Page.test.tsx`.

## Demo (`/demo/phase4`)

Snapshot mode (no API key). Set date range, sliders → **Recompute** → table updates. Select a window for Integrity & Evidence; **Resolve Dispute** only when Compare = MATCH and status = Disputed.

- **Proxy (BYOK):** `npm run demo:server` (port 3001); Vite proxies `/api` there. `VITE_DEMO_BYOK_ENABLED=true` shows key input; key sent as `X-Dune-Api-Key` only.
- **Real-data snapshot:** `frontend/server/refresh_demo_snapshot.sh` (uses `DUNE_API_KEY` from `.env`). Optional env: `DUNE_DAYS`, `DUNE_LIMIT`, `DEMO_SERVICE_ID`.

## Structure

- `src/app/` — routing, layout
- `src/pages/` — Settlements, Claims, Disputes, Policy, **DemoPhase4Page**
- `src/components/` — DataTable, StatusBadge, **DemoControlPanel**, **CompareStatusChip**, etc.
- `src/domain/` — types, status/error mapping
- `src/adapters/` — **DemoSnapshotAdapter**, **createDemoProxyAdapter**, contexts
- `server/` — **demo-proxy.mjs** (GET /api/demo/windows, cache, limits)
