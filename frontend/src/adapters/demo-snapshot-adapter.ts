/**
 * Demo snapshot adapter: loads phase4_snapshot.json and implements DemoAnalyticsAdapter.
 * No network dependency in snapshot mode; filters and recomputes client-side.
 */

import type { DemoAnalyticsAdapter } from './demo-analytics-interface';
import type {
  DemoWindowAggregate,
  DemoEvidenceView,
  ReplaySummaryView,
} from '../domain/types';

const SNAPSHOT_URL = `${import.meta.env.BASE_URL ?? '/'}demo_data/phase4_snapshot.json`;

interface SnapshotWindowRaw {
  owner: string;
  service_id: string;
  window_id: string;
  from_ts: string;
  to_ts: string;
  gross_spent: number;
  operator_share: number;
  protocol_fee: number;
  reserve_locked: number;
  top_n_share: number;
  operator_count: number;
  status?: string;
  evidence_hash?: string;
  replay_hash?: string | null;
  replay_summary?: ReplaySummaryView | null;
  from_tx_id?: number;
  to_tx_id?: number;
}

interface SnapshotPayload {
  version?: number;
  generated_at?: string;
  windows: SnapshotWindowRaw[];
  usage_rows?: unknown[];
}

function validateWindow(w: unknown): w is SnapshotWindowRaw {
  if (!w || typeof w !== 'object') return false;
  const o = w as Record<string, unknown>;
  return (
    typeof o.owner === 'string' &&
    typeof o.service_id === 'string' &&
    typeof o.window_id === 'string' &&
    typeof o.from_ts === 'string' &&
    typeof o.to_ts === 'string' &&
    typeof (o as { gross_spent?: number }).gross_spent === 'number' &&
    typeof (o as { operator_share?: number }).operator_share === 'number' &&
    typeof (o as { protocol_fee?: number }).protocol_fee === 'number' &&
    typeof (o as { reserve_locked?: number }).reserve_locked === 'number' &&
    typeof (o as { top_n_share?: number }).top_n_share === 'number' &&
    typeof (o as { operator_count?: number }).operator_count === 'number'
  );
}

/** Normalize bps so they sum to 10000; then split gross_spent with conservation (sum === gross_spent). */
function splitConserving(
  gross_spent: number,
  operator_share_bps: number,
  protocol_fee_bps: number,
  reserve_bps: number
): { operator_share: number; protocol_fee: number; reserve_locked: number } {
  const totalBps = operator_share_bps + protocol_fee_bps + reserve_bps;
  const scale = totalBps > 0 ? 10000 / totalBps : 1;
  const bpsOp = Math.round(operator_share_bps * scale);
  const bpsProto = Math.round(protocol_fee_bps * scale);
  const bpsReserve = Math.round(reserve_bps * scale);
  let operator_share = Math.floor((gross_spent * bpsOp) / 10000);
  const protocol_fee = Math.floor((gross_spent * bpsProto) / 10000);
  const reserve_locked = Math.floor((gross_spent * bpsReserve) / 10000);
  const sum = operator_share + protocol_fee + reserve_locked;
  if (sum < gross_spent) {
    operator_share += gross_spent - sum;
  } else if (sum > gross_spent) {
    operator_share -= Math.min(operator_share, sum - gross_spent);
  }
  return { operator_share, protocol_fee, reserve_locked };
}

function mapWindow(raw: SnapshotWindowRaw, policy?: {
  operator_share_bps?: number;
  protocol_fee_bps?: number;
  reserve_bps?: number;
}): DemoWindowAggregate {
  let { operator_share, protocol_fee, reserve_locked } = raw;
  if (policy && (policy.operator_share_bps != null || policy.protocol_fee_bps != null || policy.reserve_bps != null)) {
    const bpsOp = policy.operator_share_bps ?? 9000;
    const bpsProto = policy.protocol_fee_bps ?? 1000;
    const bpsReserve = policy.reserve_bps ?? 0;
    const split = splitConserving(raw.gross_spent, bpsOp, bpsProto, bpsReserve);
    operator_share = split.operator_share;
    protocol_fee = split.protocol_fee;
    reserve_locked = split.reserve_locked;
  }
  return {
    owner: raw.owner,
    service_id: raw.service_id,
    window_id: raw.window_id,
    from_ts: raw.from_ts,
    to_ts: raw.to_ts,
    gross_spent: raw.gross_spent,
    operator_share,
    protocol_fee,
    reserve_locked,
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

function inDateRange(
  from_ts: string,
  to_ts: string,
  start_date: string,
  end_date: string,
  granularity: 'day' | 'week'
): boolean {
  const from = new Date(from_ts).getTime();
  const to = new Date(to_ts).getTime();
  const start = new Date(start_date).getTime();
  const end = new Date(end_date).getTime();
  if (granularity === 'week') {
    return from <= end && to >= start;
  }
  return from <= end && to >= start;
}

let cached: SnapshotPayload | null = null;

async function loadSnapshot(): Promise<SnapshotPayload> {
  if (cached) return cached;
  const res = await fetch(SNAPSHOT_URL);
  if (!res.ok) throw new Error(`Snapshot load failed: ${res.status}`);
  const data = (await res.json()) as unknown;
  if (!data || typeof data !== 'object' || !Array.isArray((data as SnapshotPayload).windows)) {
    throw new Error('Invalid snapshot: missing windows array');
  }
  const payload = data as SnapshotPayload;
  for (const w of payload.windows) {
    if (!validateWindow(w)) throw new Error('Invalid snapshot: invalid window row');
  }
  cached = payload;
  return payload;
}

export const DemoSnapshotAdapter: DemoAnalyticsAdapter = {
  async getDemoWindows(params) {
    const payload = await loadSnapshot();
    const { start_date, end_date, owner, service_id, window_granularity, top_n } = params;
    const policy = {
      operator_share_bps: params.operator_share_bps,
      protocol_fee_bps: params.protocol_fee_bps,
      reserve_bps: params.reserve_bps,
    };
    let list = payload.windows
      .filter((w) => inDateRange(w.from_ts, w.to_ts, start_date, end_date, window_granularity))
      .filter((w) => !owner || w.owner === owner)
      .filter((w) => !service_id || w.service_id === service_id)
      .map((w) => mapWindow(w, policy));
    if (top_n != null && top_n > 0) {
      list = list.filter((w) => w.operator_count <= top_n);
    }
    return list;
  },

  async getDemoEvidence(window_id: string, owner: string, service_id: string) {
    const payload = await loadSnapshot();
    const w = payload.windows.find(
      (x) => x.window_id === window_id && x.owner === owner && x.service_id === service_id
    );
    if (!w || !w.evidence_hash || !w.replay_hash || !w.replay_summary) return null;
    const ev: DemoEvidenceView = {
      window_id: w.window_id,
      evidence_hash: w.evidence_hash,
      replay_hash: w.replay_hash,
      replay_summary: w.replay_summary,
    };
    return ev;
  },
};
