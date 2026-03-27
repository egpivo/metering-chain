import { describe, expect, it } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AdapterProvider } from '../adapters/context';
import { MockAdapter } from '../adapters/mock-adapter';
import { SettlementsPage } from './SettlementsPage';

describe('SettlementsPage', () => {
  it('renders settlements table with rows from adapter', async () => {
    render(
      <MemoryRouter>
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
});
