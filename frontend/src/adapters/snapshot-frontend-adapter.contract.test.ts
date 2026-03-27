import { describe, expect, it, vi } from 'vitest';

describe('SnapshotFrontendAdapter contract', () => {
  it('returns deterministic readonly error contract for write actions', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ windows: [] }),
    }));
    vi.resetModules();
    const { SnapshotFrontendAdapter } = await import('./snapshot-frontend-adapter');

    const calls = await Promise.all([
      SnapshotFrontendAdapter.finalizeSettlement('o', 's', 'w'),
      SnapshotFrontendAdapter.submitClaim('op', 'o', 's', 'w', 1),
      SnapshotFrontendAdapter.payClaim('op', 'o', 's', 'w'),
      SnapshotFrontendAdapter.openDispute('o', 's', 'w'),
      SnapshotFrontendAdapter.resolveDispute('o', 's', 'w', 'dismissed'),
      SnapshotFrontendAdapter.publishPolicy({
        scope: 'global',
        version: 1,
        effective_from_tx_id: 0,
        operator_share_bps: 9000,
        protocol_fee_bps: 1000,
        dispute_window_secs: 60,
      }),
    ]);

    for (const result of calls) {
      expect('error_code' in result).toBe(true);
      if ('error_code' in result) {
        expect(result.error_code).toBe('DEMO_READ_ONLY');
        expect(result.message.toLowerCase()).toContain('snapshot');
        expect(result.suggested_action.toLowerCase()).toContain('live backend');
      }
    }
  });

  it('fails fast on malformed snapshot payload', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ windows: null }),
    }));
    vi.resetModules();
    const { SnapshotFrontendAdapter } = await import('./snapshot-frontend-adapter');

    await expect(SnapshotFrontendAdapter.listSettlements()).rejects.toThrow(
      /invalid snapshot payload/i
    );
  });
});
