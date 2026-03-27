import { describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AdapterProvider } from '../adapters/context';
import { MockAdapter } from '../adapters/mock-adapter';
import { DisputesPage } from './DisputesPage';
import type { FrontendDataAdapter } from '../adapters/interface';

describe('DisputesPage', () => {
  it('loads disputed settlements and resolves per-settlement dispute details', async () => {
    const getDispute = vi.fn().mockImplementation(async (owner: string) => {
      if (owner === 'alice') {
        return { settlement_key: 'alice:storage:w1', status: 'Open', resolution_audit: null };
      }
      return null;
    });
    const adapterWithDispute: FrontendDataAdapter = {
      ...MockAdapter,
      listSettlements: async () => [
        {
          settlement_id: 'alice:storage:w1',
          owner: 'alice',
          service_id: 'storage',
          window_id: 'w1',
          status: 'Disputed',
          gross_spent: 50,
          operator_share: 45,
          protocol_fee: 5,
          reserve_locked: 0,
          payable: 45,
          total_paid: 0,
          evidence_hash: 'eh',
          from_tx_id: 0,
          to_tx_id: 3,
          replay_hash: null,
          replay_summary: null,
        },
        {
          settlement_id: 'bob:compute:w2',
          owner: 'bob',
          service_id: 'compute',
          window_id: 'w2',
          status: 'Disputed',
          gross_spent: 70,
          operator_share: 63,
          protocol_fee: 7,
          reserve_locked: 0,
          payable: 63,
          total_paid: 0,
          evidence_hash: 'eh2',
          from_tx_id: 4,
          to_tx_id: 9,
          replay_hash: null,
          replay_summary: null,
        },
      ],
      getDispute,
    };

    render(
      <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AdapterProvider adapter={adapterWithDispute}>
          <DisputesPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByRole('table')).toBeInTheDocument();
    });

    expect(getDispute).toHaveBeenCalledTimes(2);
    expect(screen.getByText('alice:storage:w1')).toBeInTheDocument();
    expect(screen.getByText('Open')).toBeInTheDocument();
    expect(screen.queryByText('bob:compute:w2')).not.toBeInTheDocument();
  });

  it('shows API error banner when listing disputes fails', async () => {
    const adapterWithError: FrontendDataAdapter = {
      ...MockAdapter,
      listSettlements: async () => {
        throw {
          error_code: 'BACKEND_DOWN',
          message: 'cannot reach api',
          suggested_action: 'retry',
        };
      },
    };

    render(
      <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AdapterProvider adapter={adapterWithError}>
          <DisputesPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('BACKEND_DOWN')).toBeInTheDocument();
    });
  });

  it('shows API error banner when dispute detail fetch fails for one settlement', async () => {
    const adapterWithDetailError: FrontendDataAdapter = {
      ...MockAdapter,
      listSettlements: async () => [
        {
          settlement_id: 'alice:storage:w1',
          owner: 'alice',
          service_id: 'storage',
          window_id: 'w1',
          status: 'Disputed',
          gross_spent: 50,
          operator_share: 45,
          protocol_fee: 5,
          reserve_locked: 0,
          payable: 45,
          total_paid: 0,
          evidence_hash: 'eh',
          from_tx_id: 0,
          to_tx_id: 3,
          replay_hash: null,
          replay_summary: null,
        },
      ],
      getDispute: async () => {
        throw {
          error_code: 'DISPUTE_LOOKUP_FAILED',
          message: 'detail query failed',
          suggested_action: 'retry dispute query',
        };
      },
    };

    render(
      <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AdapterProvider adapter={adapterWithDetailError}>
          <DisputesPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('DISPUTE_LOOKUP_FAILED')).toBeInTheDocument();
    });
    expect(screen.getByText('detail query failed')).toBeInTheDocument();
  });
});
