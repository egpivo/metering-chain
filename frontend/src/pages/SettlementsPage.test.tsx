import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AdapterProvider } from '../adapters/context';
import { MockAdapter } from '../adapters/mock-adapter';
import { SettlementsPage } from './SettlementsPage';
import type { FrontendDataAdapter } from '../adapters/interface';

describe('SettlementsPage', () => {
  it('renders settlements table with rows from adapter', async () => {
    render(
      <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AdapterProvider adapter={MockAdapter}>
          <SettlementsPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByRole('table')).toBeInTheDocument();
    });

    expect(screen.getByText('Settlements')).toBeInTheDocument();
    expect(screen.getByText('alice / w1')).toBeInTheDocument();
  });

  it('uses deep-link date params and updates adapter filters on user input', async () => {
    const listSettlements = vi.fn().mockResolvedValue([
      {
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
        evidence_hash: 'eh',
        from_tx_id: 0,
        to_tx_id: 3,
        replay_hash: null,
        replay_summary: null,
      },
    ]);
    const adapter: FrontendDataAdapter = { ...MockAdapter, listSettlements };

    render(
      <MemoryRouter
        initialEntries={['/settlements?start_date=2026-02-01&end_date=2026-02-28']}
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <AdapterProvider adapter={adapter}>
          <SettlementsPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(listSettlements).toHaveBeenCalled();
    });
    expect(listSettlements).toHaveBeenCalledWith(
      expect.objectContaining({
        start_date: '2026-02-01',
        end_date: '2026-02-28',
      })
    );

    fireEvent.change(screen.getByPlaceholderText('Filter by owner'), {
      target: { value: 'alice' },
    });

    await waitFor(() => {
      expect(listSettlements).toHaveBeenLastCalledWith(
        expect.objectContaining({
          owner: 'alice',
          start_date: '2026-02-01',
          end_date: '2026-02-28',
        })
      );
    });
  });

  it('shows error banner with deterministic API error fields', async () => {
    const adapterWithError: FrontendDataAdapter = {
      ...MockAdapter,
      listSettlements: async () => {
        throw {
          error_code: 'REPLAY_PROTOCOL_MISMATCH',
          message: 'contract changed',
          suggested_action: 'upgrade binary',
        };
      },
    };

    render(
      <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AdapterProvider adapter={adapterWithError}>
          <SettlementsPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('REPLAY_PROTOCOL_MISMATCH')).toBeInTheDocument();
    });
    expect(screen.getByText('contract changed')).toBeInTheDocument();
    expect(screen.getByText(/upgrade binary/i)).toBeInTheDocument();
  });
});
