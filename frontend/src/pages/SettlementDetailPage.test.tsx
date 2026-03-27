import { describe, expect, it } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { AdapterProvider } from '../adapters/context';
import { MockAdapter } from '../adapters/mock-adapter';
import { SettlementDetailPage } from './SettlementDetailPage';

describe('SettlementDetailPage', () => {
  it('renders settlement economics and integrity blocks', async () => {
    render(
      <MemoryRouter initialEntries={['/settlements/alice/storage/w1']}>
        <AdapterProvider adapter={MockAdapter}>
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
  });
});
