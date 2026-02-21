/**
 * Day 6+7: Demo backend proxy. Serves GET /api/demo/windows with same shape as snapshot adapter.
 * Fixed query allowlist, param validation, in-memory cache, timeout, row cap.
 * Run from repo: node frontend/server/demo-proxy.mjs (or cd frontend && node server/demo-proxy.mjs)
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import express from 'express';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SNAPSHOT_PATH = process.env.DEMO_SNAPSHOT_PATH || path.join(__dirname, '../public/demo_data/phase4_snapshot.json');
const PORT = Number(process.env.DEMO_PROXY_PORT) || 3001;
const CACHE_TTL_MS = Number(process.env.DEMO_CACHE_TTL_MS) || 5 * 60 * 1000; // 5 min
const MAX_DATE_SPAN_DAYS = Number(process.env.DEMO_MAX_DATE_SPAN_DAYS) || 365;
const MAX_ROWS = Number(process.env.DEMO_MAX_ROWS) || 500;
const REQUEST_TIMEOUT_MS = 15_000;

const ALLOWED_GRANULARITY = new Set(['day', 'week']);

const cache = new Map();
function cacheGet(key) {
  const entry = cache.get(key);
  if (!entry) return null;
  if (Date.now() > entry.expiresAt) {
    cache.delete(key);
    return null;
  }
  return entry.data;
}
function cacheSet(key, data) {
  cache.set(key, { data, expiresAt: Date.now() + CACHE_TTL_MS });
}

function splitConserving(grossSpent, operatorBps, protocolBps, reserveBps) {
  const totalBps = operatorBps + protocolBps + reserveBps;
  const scale = totalBps > 0 ? 10000 / totalBps : 1;
  const bpsOp = Math.round(operatorBps * scale);
  const bpsProto = Math.round(protocolBps * scale);
  const bpsReserve = Math.round(reserveBps * scale);
  let operatorShare = Math.floor((grossSpent * bpsOp) / 10000);
  const protocolFee = Math.floor((grossSpent * bpsProto) / 10000);
  const reserveLocked = Math.floor((grossSpent * bpsReserve) / 10000);
  const sum = operatorShare + protocolFee + reserveLocked;
  if (sum < grossSpent) operatorShare += grossSpent - sum;
  else if (sum > grossSpent) operatorShare -= Math.min(operatorShare, sum - grossSpent);
  return { operatorShare, protocolFee, reserveLocked };
}

function mapWindow(raw, policy) {
  let { operator_share: operatorShare, protocol_fee: protocolFee, reserve_locked: reserveLocked } = raw;
  if (policy && (policy.operator_share_bps != null || policy.protocol_fee_bps != null || policy.reserve_bps != null)) {
    const bpsOp = policy.operator_share_bps ?? 9000;
    const bpsProto = policy.protocol_fee_bps ?? 1000;
    const bpsReserve = policy.reserve_bps ?? 0;
    const split = splitConserving(raw.gross_spent, bpsOp, bpsProto, bpsReserve);
    operatorShare = split.operatorShare;
    protocolFee = split.protocolFee;
    reserveLocked = split.reserveLocked;
  }
  return {
    owner: raw.owner,
    service_id: raw.service_id,
    window_id: raw.window_id,
    from_ts: raw.from_ts,
    to_ts: raw.to_ts,
    gross_spent: raw.gross_spent,
    operator_share: operatorShare,
    protocol_fee: protocolFee,
    reserve_locked: reserveLocked,
    top_n_share: raw.top_n_share,
    operator_count: raw.operator_count,
    status: raw.status,
    evidence_hash: raw.evidence_hash,
    replay_hash: raw.replay_hash ?? null,
    replay_summary: raw.replay_summary ?? null,
    from_tx_id: raw.from_tx_id,
    to_tx_id: raw.to_tx_id,
  };
}

function inDateRange(fromTs, toTs, startDate, endDate) {
  const from = new Date(fromTs).getTime();
  const to = new Date(toTs).getTime();
  const start = new Date(startDate).getTime();
  const end = new Date(endDate).getTime();
  return from <= end && to >= start;
}

function loadSnapshotSync() {
  const raw = fs.readFileSync(SNAPSHOT_PATH, 'utf8');
  const data = JSON.parse(raw);
  if (!data?.windows || !Array.isArray(data.windows)) throw new Error('Invalid snapshot: missing windows array');
  return data;
}

function validateParams(q) {
  const start = q.start_date;
  const end = q.end_date;
  if (!start || !end) return { ok: false, status: 400, message: 'start_date and end_date required' };
  const startT = new Date(start).getTime();
  const endT = new Date(end).getTime();
  if (Number.isNaN(startT) || Number.isNaN(endT)) return { ok: false, status: 400, message: 'Invalid date format' };
  if (endT < startT) return { ok: false, status: 400, message: 'end_date must be >= start_date' };
  const spanDays = (endT - startT) / (24 * 60 * 60 * 1000);
  if (spanDays > MAX_DATE_SPAN_DAYS) return { ok: false, status: 400, message: `Date span must be <= ${MAX_DATE_SPAN_DAYS} days` };
  const gran = q.window_granularity || 'day';
  if (!ALLOWED_GRANULARITY.has(gran)) return { ok: false, status: 400, message: 'window_granularity must be day or week' };
  const num = (v, def, min, max) => {
    const n = v !== undefined && v !== '' ? Number(v) : def;
    if (Number.isNaN(n) || n < min || n > max) return null;
    return n;
  };
  const operator_share_bps = num(q.operator_share_bps, 9000, 0, 10000);
  const protocol_fee_bps = num(q.protocol_fee_bps, 1000, 0, 10000);
  const reserve_bps = num(q.reserve_bps, 0, 0, 10000);
  const top_n = num(q.top_n, 0, 0, 1000);
  if (operator_share_bps == null || protocol_fee_bps == null || reserve_bps == null || top_n == null) {
    return { ok: false, status: 400, message: 'Invalid number param (bps 0-10000, top_n 0-1000)' };
  }
  return {
    ok: true,
    params: {
      start_date: start,
      end_date: end,
      owner: q.owner || undefined,
      service_id: q.service_id || undefined,
      window_granularity: gran,
      operator_share_bps,
      protocol_fee_bps,
      reserve_bps,
      top_n,
    },
  };
}

function cacheKey(params) {
  return JSON.stringify(params);
}

const app = express();
app.use(express.json());

const DEMO_PROXY_PORT_FOR_CORS = Number(process.env.DEMO_PROXY_PORT) || 3001;
const allowedOrigins = new Set([
  `http://localhost:${DEMO_PROXY_PORT_FOR_CORS}`,
  'http://localhost:5173',
  'http://127.0.0.1:5173',
  'http://127.0.0.1:3001',
]);
app.use((req, res, next) => {
  const origin = req.get('Origin');
  if (origin && allowedOrigins.has(origin)) {
    res.setHeader('Access-Control-Allow-Origin', origin);
  }
  res.setHeader('Access-Control-Allow-Methods', 'GET, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, X-Dune-Api-Key');
  if (req.method === 'OPTIONS') return res.sendStatus(204);
  next();
});

app.get('/api/demo/windows', (req, res) => {
  const byokKey = req.headers['x-dune-api-key'];
  const keyProvided = Boolean(byokKey?.trim());
  const validation = validateParams(req.query);
  if (!validation.ok) {
    return res.status(validation.status).json({ error: validation.message });
  }
  const { params } = validation;
  const cacheKeyStr = cacheKey(params);
  const cached = cacheGet(cacheKeyStr);
  const meta = { mode: 'snapshot', key_provided: keyProvided, key_used: false };
  if (cached) {
    return res.json({ ...cached, _meta: meta });
  }
  try {
    const payload = loadSnapshotSync();
    const policy = {
      operator_share_bps: params.operator_share_bps,
      protocol_fee_bps: params.protocol_fee_bps,
      reserve_bps: params.reserve_bps,
    };
    let list = payload.windows
      .filter((w) => inDateRange(w.from_ts, w.to_ts, params.start_date, params.end_date))
      .filter((w) => !params.owner || w.owner === params.owner)
      .filter((w) => !params.service_id || w.service_id === params.service_id)
      .map((w) => mapWindow(w, policy));
    if (params.top_n > 0) {
      list = list.filter((w) => w.operator_count <= params.top_n);
    }
    list = list.slice(0, MAX_ROWS);
    const body = { windows: list };
    cacheSet(cacheKeyStr, body);
    res.json({ ...body, _meta: meta });
  } catch (err) {
    console.error(err);
    res.status(500).json({ error: err.message || 'Server error' });
  }
});

app.use((_req, res) => res.status(404).json({ error: 'Not found' }));

const server = app.listen(PORT, () => {
  console.log(`Demo proxy http://localhost:${PORT} (snapshot: ${SNAPSHOT_PATH})`);
});

server.timeout = REQUEST_TIMEOUT_MS;
server.keepAliveTimeout = REQUEST_TIMEOUT_MS + 1000;
