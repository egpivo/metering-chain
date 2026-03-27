import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AuditDataPage } from './AuditDataPage';

describe('AuditDataPage', () => {
  it('renders data source credit and refresh guidance', () => {
    render(
      <MemoryRouter>
        <AuditDataPage />
      </MemoryRouter>
    );

    expect(screen.getByText('Data Source')).toBeInTheDocument();
    expect(screen.getByText('Credit')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Dune' })).toBeInTheDocument();
    expect(screen.getByText(/refresh_demo_snapshot\.sh/i)).toBeInTheDocument();
  });
});
