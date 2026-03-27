import { describe, expect, it, vi } from 'vitest';

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
  status: string;
  evidence_hash: string;
  replay_hash: string | null;
  replay_summary: {
    from_tx_id: number;
    to_tx_id: number;
    tx_count: number;
    gross_spent: number;
    operator_share: number;
    protocol_fee: number;
    reserve_locked: number;
  } | null;
  from_tx_id: number;
  to_tx_id: number;
}

function buildLargeSnapshot(size: number): { windows: SnapshotWindowRaw[] } {
  const windows: SnapshotWindowRaw[] = [];
  for (let i = 0; i < size; i += 1) {
    windows.push({
      owner: `owner-${i % 100}`,
      service_id: `svc-${i % 10}`,
      window_id: `2026-02-${String((i % 28) + 1).padStart(2, '0')}`,
      from_ts: `2026-02-${String((i % 28) + 1).padStart(2, '0')}T00:00:00Z`,
      to_ts: `2026-02-${String((i % 28) + 1).padStart(2, '0')}T23:59:59Z`,
      gross_spent: 100 + (i % 50),
      operator_share: 90 + (i % 40),
      protocol_fee: 10,
      reserve_locked: 0,
      top_n_share: 90,
      operator_count: (i % 7) + 1,
      status: i % 5 === 0 ? 'Disputed' : 'Finalized',
      evidence_hash: `ev-${i}`,
      replay_hash: i % 5 === 0 ? null : `rh-${i}`,
      replay_summary:
        i % 5 === 0
          ? null
          : {
              from_tx_id: i * 3,
              to_tx_id: i * 3 + 3,
              tx_count: 3,
              gross_spent: 100 + (i % 50),
              operator_share: 90 + (i % 40),
              protocol_fee: 10,
              reserve_locked: 0,
            },
      from_tx_id: i * 3,
      to_tx_id: i * 3 + 3,
    });
  }
  return { windows };
}

describe('SnapshotFrontendAdapter perf visibility (first pass)', () => {
  it('loads a large snapshot and reuses cache without refetch', async () => {
    const payload = buildLargeSnapshot(10_000);
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => payload,
    });
    vi.stubGlobal('fetch', fetchMock);

    // Re-import module to reset its module-level cache.
    vi.resetModules();
    const { SnapshotFrontendAdapter } = await import('./snapshot-frontend-adapter');

    const t1 = performance.now();
    const first = await SnapshotFrontendAdapter.listSettlements();
    const firstElapsedMs = performance.now() - t1;

    const t2 = performance.now();
    const second = await SnapshotFrontendAdapter.listSettlements({ owner: 'owner-1' });
    const secondElapsedMs = performance.now() - t2;

    expect(first).toHaveLength(10_000);
    expect(second.length).toBeGreaterThan(0);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    // Reporting-only signal for local/CI logs (not a hard threshold gate yet).
    console.info(
      `[snapshot_perf] first_load_ms=${firstElapsedMs.toFixed(2)} cached_query_ms=${secondElapsedMs.toFixed(2)} windows=${first.length}`
    );
  });

  it('measures metering adapter path on realistic large snapshot payload', async () => {
    const payload = buildLargeSnapshot(10_000);
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => payload,
    });
    vi.stubGlobal('fetch', fetchMock);

    vi.resetModules();
    const { MeteringSnapshotAdapter } = await import('./metering-snapshot-adapter');

    const t1 = performance.now();
    const series = await MeteringSnapshotAdapter.getMeteringSeries({
      start_date: '2026-02-01',
      end_date: '2026-02-28',
      granularity: 'day',
    });
    const firstElapsedMs = performance.now() - t1;

    const t2 = performance.now();
    const counters = await MeteringSnapshotAdapter.getMeteringCounters({
      start_date: '2026-02-01',
      end_date: '2026-02-28',
    });
    const secondElapsedMs = performance.now() - t2;

    expect(series.length).toBeGreaterThan(0);
    expect(counters.windows_in_range).toBe(10_000);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    console.info(
      `[snapshot_perf_metering] series_ms=${firstElapsedMs.toFixed(2)} counters_ms=${secondElapsedMs.toFixed(2)} windows=${counters.windows_in_range}`
    );
  });
});
