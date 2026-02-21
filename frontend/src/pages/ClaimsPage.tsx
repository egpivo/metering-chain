import { useState, useEffect } from 'react';
import { useAdapter } from '../adapters/context';
import { DataTable, type Column } from '../components/DataTable';
import { StatusBadge } from '../components/StatusBadge';
import { ErrorBanner } from '../components/ErrorBanner';
import type { ClaimView } from '../domain/types';

export function ClaimsPage() {
  const adapter = useAdapter();
  const isReadOnly = adapter.readonlyMode === true;
  const [list, setList] = useState<ClaimView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<{ error_code: string; message: string; suggested_action: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    adapter
      .listClaims()
      .then((data) => { if (!cancelled) setList(data); })
      .catch((e: unknown) => {
        if (!cancelled && e && typeof e === 'object' && 'error_code' in e) {
          setError(e as { error_code: string; message: string; suggested_action: string });
        } else {
          setError({ error_code: 'UNKNOWN', message: String(e), suggested_action: 'Retry.' });
        }
      })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [adapter]);

  const columns: Column<ClaimView>[] = [
    { key: 'claim_id', label: 'Claim ID' },
    { key: 'operator', label: 'Operator' },
    { key: 'status', label: 'Status', render: (r) => <StatusBadge kind="claim" status={r.status} /> },
    { key: 'claim_amount', label: 'Claim amount' },
    { key: 'settlement_key', label: 'Settlement', render: (r) => r.settlement_key ?? '—' },
  ];

  const paidToDate = list.filter((c) => c.status?.toLowerCase().includes('paid')).reduce((s, c) => s + c.claim_amount, 0);
  const remaining = list.filter((c) => c.status?.toLowerCase().includes('pending')).reduce((s, c) => s + c.claim_amount, 0);

  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-4)' }}>Claims</h1>
      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
      {isReadOnly && list.length === 0 && !loading && (
        <div className="card" style={{ marginBottom: 'var(--space-4)' }}>
          <h3>Snapshot Mode</h3>
          <p style={{ margin: 0, color: 'var(--color-text-muted)' }}>
            Claims are not included in the current snapshot dataset yet.
          </p>
        </div>
      )}
      <div className="card" style={{ marginBottom: 'var(--space-4)' }}>
        <h3>Summary</h3>
        <p>Paid to date: {paidToDate}</p>
        <p>Remaining claimable: {remaining}</p>
      </div>
      {loading ? (
        <p style={{ color: 'var(--color-text-muted)' }}>Loading…</p>
      ) : (
        !isReadOnly || list.length > 0 ? (
          <DataTable columns={columns} data={list} keyFn={(r) => r.claim_id} emptyMessage="No claims" />
        ) : null
      )}
    </div>
  );
}
