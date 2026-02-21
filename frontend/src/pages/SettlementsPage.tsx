import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { useAdapter } from '../adapters/context';
import { DataTable, type Column } from '../components/DataTable';
import { StatusBadge } from '../components/StatusBadge';
import { ErrorBanner } from '../components/ErrorBanner';
import { formatInteger } from '../utils/format';
import type { SettlementView } from '../domain/types';

function settlementShortLabel(r: SettlementView): string {
  const prefix = r.owner.length > 10 ? `${r.owner.slice(0, 8)}…` : r.owner;
  return `${prefix} / ${r.window_id}`;
}

export function SettlementsPage() {
  const adapter = useAdapter();
  const [list, setList] = useState<SettlementView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<{ error_code: string; message: string; suggested_action: string } | null>(null);
  const [owner, setOwner] = useState('');
  const [serviceId, setServiceId] = useState('');
  const [status, setStatus] = useState('');

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    adapter
      .listSettlements({ owner: owner || undefined, service_id: serviceId || undefined, status: status || undefined })
      .then((data) => {
        if (!cancelled) setList(data);
      })
      .catch((e: unknown) => {
        if (!cancelled && e && typeof e === 'object' && 'error_code' in e) {
          setError(e as { error_code: string; message: string; suggested_action: string });
        } else {
          setError({ error_code: 'UNKNOWN', message: String(e), suggested_action: 'Retry or check backend.' });
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => { cancelled = true; };
  }, [adapter, owner, serviceId, status]);

  const columns: Column<SettlementView>[] = [
    {
      key: 'settlement_id',
      label: 'Settlement',
      width: '16%',
      render: (r) => (
        <Link to={`/settlements/${r.owner}/${r.service_id}/${r.window_id}`} title={r.settlement_id}>
          <span className="cell-nowrap">{settlementShortLabel(r)}</span>
        </Link>
      ),
    },
    { key: 'owner', label: 'Owner', width: '12%', render: (r) => <span className="cell-nowrap">{r.owner.length > 12 ? `${r.owner.slice(0, 10)}…` : r.owner}</span> },
    { key: 'service_id', label: 'Service', width: '9%', render: (r) => <span className="cell-service"><span className="service-badge">{r.service_id}</span></span> },
    { key: 'window_id', label: 'Window', width: '9%', render: (r) => <span className="cell-nowrap">{r.window_id}</span> },
    { key: 'status', label: 'Status', width: '9%', render: (r) => <StatusBadge kind="settlement" status={r.status} /> },
    { key: 'gross_spent', label: 'Gross spent', width: '18%', render: (r) => <span className="cell-num">{formatInteger(r.gross_spent)}</span> },
    { key: 'operator_share', label: 'Operator share', width: '18%', render: (r) => <span className="cell-num">{formatInteger(r.operator_share)}</span> },
    { key: 'payable', label: 'Payable', width: '9%', render: (r) => <span className="cell-num">{formatInteger(r.payable)}</span> },
  ];

  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-4)' }}>Settlements</h1>
      <p style={{ color: 'var(--color-text-muted)', marginBottom: 'var(--space-3)' }}>
        DePIN Helium IOT settlement windows derived from Dune transfer data snapshots.
      </p>
      <div className="card" style={{ marginBottom: 'var(--space-4)' }}>
        <h3>Filters</h3>
        <div style={{ display: 'flex', gap: 'var(--space-4)', flexWrap: 'wrap' }}>
          <label>
            Owner <input value={owner} onChange={(e) => setOwner(e.target.value)} placeholder="Filter by owner" />
          </label>
          <label>
            Service ID <input value={serviceId} onChange={(e) => setServiceId(e.target.value)} placeholder="Filter by service" />
          </label>
          <label>
            Status <input value={status} onChange={(e) => setStatus(e.target.value)} placeholder="e.g. Finalized" />
          </label>
        </div>
      </div>
      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
      {loading ? <p style={{ color: 'var(--color-text-muted)' }}>Loading…</p> : (
        <div style={{ overflowX: 'auto' }}>
          <DataTable columns={columns} data={list} keyFn={(r) => r.settlement_id} emptyMessage="No settlements" tableClassName="data-table--fixed" />
        </div>
      )}
    </div>
  );
}
