/**
 * Metering adapter backed by phase4_snapshot.json.
 * Derives series, top operators, window preview, and counters from the windows array.
 */

import type { MeteringAdapter } from '../domain/types';
import type {
  MeteringSeriesPoint,
  MeteringTopOperator,
  MeteringWindowPreview,
  MeteringAnomalyItem,
} from '../domain/types';

const SNAPSHOT_URL = '/demo_data/phase4_snapshot.json';

interface SnapshotWindowRaw {
  owner: string;
  service_id: string;
  window_id: string;
  from_ts: string;
  to_ts: string;
  gross_spent: number;
  operator_count: number;
  from_tx_id?: number;
  to_tx_id?: number;
  status?: string;
  replay_summary?: unknown;
  replay_hash?: string | null;
}

interface SnapshotPayload {
  windows: SnapshotWindowRaw[];
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
  cached = data as SnapshotPayload;
  return cached;
}

function inDateRange(
  from_ts: string,
  to_ts: string,
  start_date: string,
  end_date: string
): boolean {
  const from = new Date(from_ts).getTime();
  const to = new Date(to_ts).getTime();
  const start = new Date(start_date).getTime();
  const end = new Date(end_date).getTime();
  return from <= end && to >= start;
}

/** Day key from ISO date string or window_id (YYYY-MM-DD). */
function dayKey(ts: string): string {
  return ts.slice(0, 10);
}

/** Week key (Monday-based) from ts. */
function weekKey(ts: string): string {
  const d = new Date(ts);
  const day = d.getUTCDay();
  const monday = new Date(d);
  monday.setUTCDate(d.getUTCDate() - (day === 0 ? 6 : day - 1));
  return monday.toISOString().slice(0, 10);
}

export const MeteringSnapshotAdapter: MeteringAdapter = {
  async getMeteringSeries(params) {
    const payload = await loadSnapshot();
    const { start_date, end_date, granularity, service_id } = params;
    const windows = payload.windows
      .filter((w) => inDateRange(w.from_ts, w.to_ts, start_date, end_date))
      .filter((w) => !service_id || w.service_id === service_id);

    const keyFn = granularity === 'week' ? weekKey : dayKey;
    const byBucket = new Map<string, { cost: number; window_count: number; owner_set: Set<string> }>();

    for (const w of windows) {
      const key = keyFn(w.from_ts);
      const cur = byBucket.get(key) ?? { cost: 0, window_count: 0, owner_set: new Set<string>() };
      cur.cost += w.gross_spent;
      cur.window_count += 1;
      cur.owner_set.add(w.owner);
      byBucket.set(key, cur);
    }

    const points: MeteringSeriesPoint[] = [];
    for (const [ts, agg] of byBucket.entries()) {
      points.push({
        ts: granularity === 'day' ? `${ts}T00:00:00Z` : `${ts}T00:00:00Z`,
        units: 0,
        cost: agg.cost,
        owner_count: agg.owner_set.size,
        window_count: agg.window_count,
      });
    }
    points.sort((a, b) => a.ts.localeCompare(b.ts));
    return points;
  },

  async getMeteringTopOperators(params) {
    const payload = await loadSnapshot();
    const { start_date, end_date, limit = 10, service_id } = params;
    const windows = payload.windows
      .filter((w) => inDateRange(w.from_ts, w.to_ts, start_date, end_date))
      .filter((w) => !service_id || w.service_id === service_id);

    const key = (o: string, s: string) => `${o}\t${s}`;
    const byOwnerService = new Map<string, { owner: string; service_id: string; cost: number; window_count: number }>();
    for (const w of windows) {
      const k = key(w.owner, w.service_id);
      const cur = byOwnerService.get(k);
      if (cur) {
        cur.cost += w.gross_spent;
        cur.window_count += 1;
      } else {
        byOwnerService.set(k, {
          owner: w.owner,
          service_id: w.service_id,
          cost: w.gross_spent,
          window_count: 1,
        });
      }
    }

    const list: MeteringTopOperator[] = Array.from(byOwnerService.values())
      .map((v) => ({
        owner: v.owner,
        service_id: v.service_id,
        units: 0,
        cost: v.cost,
        window_count: v.window_count,
      }))
      .sort((a, b) => b.cost - a.cost)
      .slice(0, limit);
    return list;
  },

  async getWindowPreview(params) {
    const payload = await loadSnapshot();
    const { start_date, end_date, service_id } = params;
    const windows = payload.windows
      .filter((w) => inDateRange(w.from_ts, w.to_ts, start_date, end_date))
      .filter((w) => !service_id || w.service_id === service_id);

    const previews: MeteringWindowPreview[] = windows.slice(0, 10).map((w) => ({
      window_id: w.window_id,
      from_ts: w.from_ts,
      to_ts: w.to_ts,
      usage_count: w.to_tx_id != null && w.from_tx_id != null ? Math.max(0, w.to_tx_id - w.from_tx_id) : 0,
      operator_count: w.operator_count,
      gross_spent: w.gross_spent,
      owner: w.owner,
      service_id: w.service_id,
    }));

    return { count: windows.length, windows: previews };
  },

  async getMeteringCounters(params) {
    const payload = await loadSnapshot();
    const { start_date, end_date, service_id } = params;
    const windows = payload.windows
      .filter((w) => inDateRange(w.from_ts, w.to_ts, start_date, end_date))
      .filter((w) => !service_id || w.service_id === service_id);

    const total_units = 0;
    const active_operators = new Set(windows.map((w) => w.owner)).size;
    const totalCost = windows.reduce((s, w) => s + w.gross_spent, 0);

    const anomaly_items: MeteringAnomalyItem[] = [];
    for (const w of windows) {
      const status = (w.status ?? '').toLowerCase();
      const isDisputed = status.includes('disputed');
      const missingReplay = w.replay_summary == null && w.replay_hash == null;
      const id = `${w.owner}-${w.service_id}-${w.window_id}`;
      if (isDisputed) {
        anomaly_items.push({
          id: `${id}-disputed`,
          kind: 'disputed',
          label: `${w.window_id} (${w.owner.slice(0, 8)}…) — disputed`,
          window_id: w.window_id,
          owner: w.owner,
          service_id: w.service_id,
        });
      } else if (missingReplay && (status.includes('proposed') || status.includes('finalized'))) {
        anomaly_items.push({
          id: `${id}-replay_gap`,
          kind: 'replay_gap',
          label: `${w.window_id} (${w.owner.slice(0, 8)}…) — no replay`,
          window_id: w.window_id,
          owner: w.owner,
          service_id: w.service_id,
        });
      }
    }
    const anomalies = anomaly_items.length;

    return {
      total_units,
      active_operators,
      windows_in_range: windows.length,
      anomalies,
      total_cost: totalCost,
      anomaly_items,
    };
  },
};
