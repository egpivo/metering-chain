import { describe, expect, it, vi } from 'vitest';

describe('MeteringSnapshotAdapter contract', () => {
  it('fails with deterministic message when snapshot fetch is non-200', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
      })
    );
    vi.resetModules();
    const { MeteringSnapshotAdapter } = await import('./metering-snapshot-adapter');

    await expect(
      MeteringSnapshotAdapter.getMeteringSeries({
        start_date: '2026-02-01',
        end_date: '2026-02-28',
        granularity: 'day',
      })
    ).rejects.toThrow('Snapshot load failed: 404');
  });

  it('fails fast on malformed payload shape', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ windows: null }),
      })
    );
    vi.resetModules();
    const { MeteringSnapshotAdapter } = await import('./metering-snapshot-adapter');

    await expect(
      MeteringSnapshotAdapter.getMeteringCounters({
        start_date: '2026-02-01',
        end_date: '2026-02-28',
      })
    ).rejects.toThrow('Invalid snapshot: missing windows array');
  });

  it('emits anomaly items for disputed and replay-gap windows', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          windows: [
            {
              owner: 'alice',
              service_id: 'storage',
              window_id: '2026-02-01',
              from_ts: '2026-02-01T00:00:00Z',
              to_ts: '2026-02-01T23:59:59Z',
              gross_spent: 10,
              operator_count: 1,
              status: 'Disputed',
              replay_hash: null,
              replay_summary: null,
            },
            {
              owner: 'bob',
              service_id: 'compute',
              window_id: '2026-02-02',
              from_ts: '2026-02-02T00:00:00Z',
              to_ts: '2026-02-02T23:59:59Z',
              gross_spent: 20,
              operator_count: 2,
              status: 'Finalized',
              replay_hash: null,
              replay_summary: null,
            },
          ],
        }),
      })
    );
    vi.resetModules();
    const { MeteringSnapshotAdapter } = await import('./metering-snapshot-adapter');

    const counters = await MeteringSnapshotAdapter.getMeteringCounters({
      start_date: '2026-02-01',
      end_date: '2026-02-28',
    });

    expect(counters.anomalies).toBe(2);
    expect(counters.anomaly_items).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          window_id: '2026-02-01',
          owner: 'alice',
          service_id: 'storage',
          kind: 'disputed',
        }),
        expect.objectContaining({
          window_id: '2026-02-02',
          owner: 'bob',
          service_id: 'compute',
          kind: 'replay_gap',
        }),
      ])
    );
  });
});
