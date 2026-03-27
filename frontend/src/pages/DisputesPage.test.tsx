import { describe, expect, it } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AdapterProvider } from '../adapters/context';
import { MockAdapter } from '../adapters/mock-adapter';
import { DisputesPage } from './DisputesPage';
import type { FrontendDataAdapter } from '../adapters/interface';

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
  ],
  getDispute: async () => ({
    settlement_key: 'alice:storage:w1',
    status: 'Open',
    resolution_audit: null,
  }),
};

describe('DisputesPage', () => {
  it('renders disputed settlement row when adapter returns open dispute', async () => {
    render(
      <MemoryRouter>
        <AdapterProvider adapter={adapterWithDispute}>
          <DisputesPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByRole('table')).toBeInTheDocument();
    });

    expect(screen.getByText('alice:storage:w1')).toBeInTheDocument();
    expect(screen.getByText('Open')).toBeInTheDocument();
  });
});
