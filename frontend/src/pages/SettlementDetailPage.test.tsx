import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { AdapterProvider } from '../adapters/context';
import { SettlementDetailPage } from './SettlementDetailPage';
import { MockAdapter } from '../adapters/mock-adapter';
import type { FrontendDataAdapter } from '../adapters/interface';

describe('SettlementDetailPage', () => {
  it('transitions proposed -> finalized and surfaces action/evidence integrity states', async () => {
    let currentStatus = 'Proposed';
    const getSettlement = vi.fn().mockImplementation(async () => ({
      settlement_id: 'alice:storage:w1',
      owner: 'alice',
      service_id: 'storage',
      window_id: 'w1',
      status: currentStatus,
      gross_spent: 999,
      operator_share: 45,
      protocol_fee: 5,
      reserve_locked: 0,
      payable: 45,
      total_paid: 0,
      evidence_hash: 'ev_hash_1',
      from_tx_id: 0,
      to_tx_id: 3,
      replay_hash: 'replay_hash_1',
      replay_summary: {
        from_tx_id: 0,
        to_tx_id: 3,
        tx_count: 3,
        gross_spent: 999,
        operator_share: 45,
        protocol_fee: 5,
        reserve_locked: 0,
      },
    }));
    const finalizeSettlement = vi.fn().mockImplementation(async () => {
      currentStatus = 'Finalized';
      return { ok: true as const };
    });
    const openDispute = vi.fn().mockResolvedValue({
      error_code: 'DISPUTE_WINDOW_CLOSED',
      message: 'window closed',
      suggested_action: 'open next window',
    });
    const adapter: FrontendDataAdapter = {
      ...MockAdapter,
      getSettlement,
      finalizeSettlement,
      openDispute,
      getEvidenceBundle: async () => ({
        settlement_key: 'alice:storage:w1',
        from_tx_id: 0,
        to_tx_id: 3,
        evidence_hash: 'ev_hash_1',
        replay_hash: 'replay_hash_1',
        replay_summary: {
          from_tx_id: 0,
          to_tx_id: 3,
          tx_count: 3,
          gross_spent: 100,
          operator_share: 45,
          protocol_fee: 5,
          reserve_locked: 0,
        },
      }),
    };

    render(
      <MemoryRouter
        initialEntries={['/settlements/alice/storage/w1']}
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <AdapterProvider adapter={adapter}>
          <Routes>
            <Route
              path="/settlements/:owner/:serviceId/:windowId"
              element={<SettlementDetailPage />}
            />
          </Routes>
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText(/Settlement: alice:storage:w1/i)).toBeInTheDocument();
    });

    expect(screen.getByText('Economics')).toBeInTheDocument();
    expect(screen.getByText('Integrity')).toBeInTheDocument();
    expect(screen.getByText('Actions')).toBeInTheDocument();
    expect(screen.getByText(/999 \(recorded\)/)).toBeInTheDocument();

    const finalizeBtn = screen.getByRole('button', { name: 'Finalize Settlement' });
    const disputeBtn = screen.getByRole('button', { name: 'Open Dispute' });
    expect(finalizeBtn).toBeEnabled();
    expect(disputeBtn).toBeDisabled();

    fireEvent.click(finalizeBtn);
    await waitFor(() => {
      expect(finalizeSettlement).toHaveBeenCalledTimes(1);
      expect(disputeBtn).toBeEnabled();
    });

    fireEvent.click(disputeBtn);
    await waitFor(() => {
      expect(openDispute).toHaveBeenCalledTimes(1);
      expect(screen.getByText('DISPUTE_WINDOW_CLOSED')).toBeInTheDocument();
    });
  });

  it('shows not found state when settlement lookup returns null', async () => {
    const adapter: FrontendDataAdapter = {
      ...MockAdapter,
      getSettlement: async () => null,
      getEvidenceBundle: async () => null,
    };

    render(
      <MemoryRouter
        initialEntries={['/settlements/alice/storage/w-missing']}
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <AdapterProvider adapter={adapter}>
          <Routes>
            <Route
              path="/settlements/:owner/:serviceId/:windowId"
              element={<SettlementDetailPage />}
            />
          </Routes>
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('Settlement not found.')).toBeInTheDocument();
    });
  });

  it('degrades gracefully when evidence endpoint returns ApiErrorView object', async () => {
    const adapter: FrontendDataAdapter = {
      ...MockAdapter,
      getSettlement: async () => ({
        settlement_id: 'alice:storage:w1',
        owner: 'alice',
        service_id: 'storage',
        window_id: 'w1',
        status: 'Finalized',
        gross_spent: 50,
        operator_share: 45,
        protocol_fee: 5,
        reserve_locked: 0,
        payable: 45,
        total_paid: 0,
        evidence_hash: 'ev_hash_1',
        from_tx_id: 0,
        to_tx_id: 3,
        replay_hash: 'replay_hash_1',
        replay_summary: {
          from_tx_id: 0,
          to_tx_id: 3,
          tx_count: 3,
          gross_spent: 50,
          operator_share: 45,
          protocol_fee: 5,
          reserve_locked: 0,
        },
      }),
      getEvidenceBundle: async () => ({
        error_code: 'EVIDENCE_NOT_FOUND',
        message: 'not stored',
        suggested_action: 'resolve dispute first',
      }),
    };

    render(
      <MemoryRouter
        initialEntries={['/settlements/alice/storage/w1']}
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <AdapterProvider adapter={adapter}>
          <Routes>
            <Route
              path="/settlements/:owner/:serviceId/:windowId"
              element={<SettlementDetailPage />}
            />
          </Routes>
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText(/Settlement: alice:storage:w1/i)).toBeInTheDocument();
    });

    expect(screen.getByText('Integrity')).toBeInTheDocument();
    expect(screen.queryByText('Evidence & Replay (G4)')).not.toBeInTheDocument();
  });
});
