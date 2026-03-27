import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AdapterProvider } from '../adapters/context';
import { PolicyPage } from './PolicyPage';
import { MockAdapter } from '../adapters/mock-adapter';
import type { FrontendDataAdapter } from '../adapters/interface';

describe('PolicyPage', () => {
  it('applies scope filter and reloads policy list', async () => {
    const listPolicies = vi.fn().mockImplementation(async (filters?: { scope?: string }) => {
      if (filters?.scope === 'global') {
        return [
          {
            scope_key: 'global',
            version: 1,
            effective_from_tx_id: 0,
            status: 'Published',
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
            dispute_window_secs: 86400,
          },
        ];
      }
      return [];
    });
    const adapter: FrontendDataAdapter = { ...MockAdapter, listPolicies };

    render(
      <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AdapterProvider adapter={adapter}>
          <PolicyPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(listPolicies).toHaveBeenCalled();
    });

    fireEvent.change(screen.getByPlaceholderText('e.g. global'), { target: { value: 'global' } });

    await waitFor(() => {
      expect(listPolicies).toHaveBeenLastCalledWith({ scope: 'global' });
      expect(screen.getByText('Published')).toBeInTheDocument();
    });
  });

  it('hides publish actions in readonly snapshot mode', async () => {
    const readonlyAdapter: FrontendDataAdapter = { ...MockAdapter, readonlyMode: true };
    render(
      <MemoryRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <AdapterProvider adapter={readonlyAdapter}>
          <PolicyPage />
        </AdapterProvider>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByText('Snapshot Mode')).toBeInTheDocument();
    });
    expect(screen.queryByText('Publish policy')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Publish new version' })).not.toBeInTheDocument();
  });
});
