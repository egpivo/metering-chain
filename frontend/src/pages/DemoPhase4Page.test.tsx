/**
 * Day 9 integration tests: Phase 4 Demo page with mock adapter.
 * 1. Slider recompute updates table deterministically
 * 2. Mismatch blocks resolve CTA
 * 3. Missing evidence shows deterministic error code (EVIDENCE_NOT_FOUND)
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, within, waitFor, fireEvent } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { DemoAdapterProvider } from '../adapters/demo-context';
import { DemoPhase4Page } from './DemoPhase4Page';
import type { DemoAnalyticsAdapter } from '../adapters/demo-analytics-interface';
import type { DemoWindowAggregate } from '../domain/types';

vi.mock('../config/demo-feature', () => ({
  DEMO_BYOK_ENABLED: false,
  DEMO_PROXY_BASE: '',
}));

const baseWindow: DemoWindowAggregate = {
  owner: 'test-owner',
  service_id: 'svc',
  window_id: '2026-02-01',
  from_ts: '2026-02-01T00:00:00Z',
  to_ts: '2026-02-02T00:00:00Z',
  gross_spent: 100,
  operator_share: 90,
  protocol_fee: 10,
  reserve_locked: 0,
  top_n_share: 90,
  operator_count: 2,
};

function wrap(adapter: DemoAnalyticsAdapter) {
  return (
    <BrowserRouter>
      <DemoAdapterProvider adapter={adapter}>
        <DemoPhase4Page />
      </DemoAdapterProvider>
    </BrowserRouter>
  );
}

describe('DemoPhase4Page', () => {
  describe('slider recompute updates table deterministically', () => {
    it('after changing control and Recompute, table reflects adapter response', async () => {
      const callLog: unknown[] = [];
      const mockAdapter: DemoAnalyticsAdapter = {
        getDemoWindows: vi.fn().mockImplementation(async (params) => {
          callLog.push(params);
          const count = callLog.length;
          return [
            {
              ...baseWindow,
              window_id: `w-${count}`,
              gross_spent: count * 100,
              operator_share: count * 90,
              protocol_fee: count * 10,
            },
          ];
        }),
        getDemoEvidence: vi.fn().mockResolvedValue(null),
      };

      render(wrap(mockAdapter));

      await waitFor(() => {
        expect(screen.queryByText('Loading…')).not.toBeInTheDocument();
      });
      await waitFor(() => {
        expect(screen.getByRole('table')).toBeInTheDocument();
      });

      const table = screen.getByRole('table');
      expect(within(table).getByText('100')).toBeInTheDocument();

      const recomputeBtn = screen.getByRole('button', { name: /Recompute/i });
      fireEvent.click(recomputeBtn);

      await waitFor(() => {
        expect(within(table).getByText('200')).toBeInTheDocument();
      });
      expect(mockAdapter.getDemoWindows).toHaveBeenCalledTimes(2);
    });
  });

  describe('mismatch blocks resolve CTA', () => {
    it('when compare is MISMATCH, Resolve Dispute is disabled', async () => {
      const windowMismatch: DemoWindowAggregate = {
        ...baseWindow,
        status: 'disputed',
        evidence_hash: 'eh',
        replay_hash: 'rh',
        replay_summary: {
          from_tx_id: 1,
          to_tx_id: 10,
          tx_count: 9,
          gross_spent: 99,
          operator_share: 89,
          protocol_fee: 10,
          reserve_locked: 0,
        },
      };
      const mockAdapter: DemoAnalyticsAdapter = {
        getDemoWindows: vi.fn().mockResolvedValue([windowMismatch]),
        getDemoEvidence: vi.fn().mockResolvedValue(null),
      };

      render(wrap(mockAdapter));

      await waitFor(() => {
        expect(screen.getByRole('table')).toBeInTheDocument();
      });

      const windowButton = screen.getByRole('button', { name: '2026-02-01' });
      fireEvent.click(windowButton);

      await waitFor(() => {
        expect(screen.getByText(/Integrity & Evidence/i)).toBeInTheDocument();
      });

      const resolveBtn = screen.getByRole('button', { name: /Resolve Dispute/i });
      expect(resolveBtn).toBeDisabled();
    });
  });

  describe('missing evidence shows deterministic error code', () => {
    it('when compare is MISSING, shows EVIDENCE_NOT_FOUND', async () => {
      const windowMissing: DemoWindowAggregate = {
        ...baseWindow,
        replay_summary: undefined,
        replay_hash: undefined,
        evidence_hash: undefined,
      };
      const mockAdapter: DemoAnalyticsAdapter = {
        getDemoWindows: vi.fn().mockResolvedValue([windowMissing]),
        getDemoEvidence: vi.fn().mockResolvedValue(null),
      };

      render(wrap(mockAdapter));

      await waitFor(() => {
        expect(screen.getByRole('table')).toBeInTheDocument();
      });

      const windowButton = screen.getByRole('button', { name: '2026-02-01' });
      fireEvent.click(windowButton);

      await waitFor(() => {
        expect(screen.getByText(/EVIDENCE_NOT_FOUND/)).toBeInTheDocument();
      });
      expect(screen.getByText(/Resolve is blocked\. \(EVIDENCE_NOT_FOUND\)/)).toBeInTheDocument();
    });
  });
});
