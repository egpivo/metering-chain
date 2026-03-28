import { useState, useEffect } from 'react';
import { useAdapter } from '../adapters/context';
import { DataTable, type Column } from '../components/DataTable';
import { StatusBadge } from '../components/StatusBadge';
import { ErrorBanner } from '../components/ErrorBanner';
import type { DisputeView } from '../domain/types';

type DisputeRow = DisputeView & { dispute_id: string };

export function DisputesPage() {
  const adapter = useAdapter();
  const [list, setList] = useState<DisputeRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<{ error_code: string; message: string; suggested_action: string } | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    adapter
      .listSettlements({ status: 'disputed' })
      .then((settlements) =>
        Promise.all(
          settlements.map((s) =>
            adapter.getDispute(s.owner, s.service_id, s.window_id).then((d) =>
              d ? ({ ...d, dispute_id: d.settlement_key } as DisputeRow) : null
            )
          )
        )
      )
      .then((rows) => {
        setList(rows.filter((r): r is DisputeRow => r !== null));
      })
      .catch((e: unknown) => {
        if (e && typeof e === 'object' && 'error_code' in e) setError(e as { error_code: string; message: string; suggested_action: string });
        else setError({ error_code: 'UNKNOWN', message: String(e), suggested_action: 'Retry.' });
      })
      .finally(() => setLoading(false));
  }, [adapter]);

  const columns: Column<DisputeRow>[] = [
    { key: 'dispute_id', label: 'Dispute (settlement)' },
    { key: 'status', label: 'Status', render: (r) => <StatusBadge kind="dispute" status={r.status} /> },
  ];

  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-4)' }}>Disputes</h1>
      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
      {!error &&
        (loading ? (
          <p style={{ color: 'var(--color-text-muted)' }}>Loading…</p>
        ) : (
          <DataTable columns={columns} data={list} keyFn={(r) => r.dispute_id} emptyMessage="No open disputes" />
        ))}
    </div>
  );
}
