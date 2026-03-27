import { describe, expect, it } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { AuditDataPage } from './AuditDataPage';

describe('AuditDataPage', () => {
  it('links to overview data-source block and exposes refresh prerequisites', () => {
    render(
      <MemoryRouter
        initialEntries={['/audit/data']}
        future={{ v7_startTransition: true, v7_relativeSplatPath: true }}
      >
        <Routes>
          <Route path="/audit/data" element={<AuditDataPage />} />
          <Route path="/overview" element={<h1>Overview</h1>} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText('Data Source')).toBeInTheDocument();
    expect(screen.getByText('Credit')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Dune' })).toHaveAttribute('href', 'https://dune.com');
    expect(screen.getByText('DUNE_API_KEY')).toBeInTheDocument();
    expect(screen.getByText(/refresh_demo_snapshot\.sh/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('link', { name: /Overview page \(Data Source block\)/i }));
    expect(screen.getByText('Overview')).toBeInTheDocument();
  });
});
