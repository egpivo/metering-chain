import { useState, useEffect } from 'react';
import { useAdapter } from '../adapters/context';
import { DataTable, type Column } from '../components/DataTable';
import { StatusBadge } from '../components/StatusBadge';
import { ErrorBanner } from '../components/ErrorBanner';
import { ActionPanel } from '../components/ActionPanel';
import type { PolicyVersionView } from '../domain/types';

export function PolicyPage() {
  const adapter = useAdapter();
  const isReadOnly = adapter.readonlyMode === true;
  const [list, setList] = useState<PolicyVersionView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<{ error_code: string; message: string; suggested_action: string } | null>(null);
  const [scopeFilter, setScopeFilter] = useState('');

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    adapter
      .listPolicies({ scope: scopeFilter || undefined })
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
  }, [adapter, scopeFilter]);

  const columns: Column<PolicyVersionView>[] = [
    { key: 'scope_key', label: 'Scope' },
    { key: 'version', label: 'Version' },
    { key: 'effective_from_tx_id', label: 'Effective from tx_id' },
    { key: 'status', label: 'Status', render: (r) => <StatusBadge kind="policy" status={r.status} /> },
    { key: 'operator_share_bps', label: 'Operator share (bps)', render: (r) => r.operator_share_bps ?? '—' },
    { key: 'protocol_fee_bps', label: 'Protocol fee (bps)', render: (r) => r.protocol_fee_bps ?? '—' },
    { key: 'dispute_window_secs', label: 'Dispute window (s)', render: (r) => r.dispute_window_secs ?? '—' },
  ];

  const handlePublish = () => {
    adapter.publishPolicy({
      scope: 'global',
      version: 1,
      effective_from_tx_id: 0,
      operator_share_bps: 9000,
      protocol_fee_bps: 1000,
      dispute_window_secs: 86400,
    }).then((r) => {
      if ('ok' in r && r.ok) adapter.listPolicies().then(setList);
      else setError(r as { error_code: string; message: string; suggested_action: string });
    }).catch((e: unknown) => setError(e && typeof e === 'object' && 'error_code' in e ? (e as { error_code: string; message: string; suggested_action: string }) : { error_code: 'UNKNOWN', message: String(e), suggested_action: '' }));
  };

  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-4)' }}>Policy</h1>
      {!isReadOnly && (
        <div className="card" style={{ marginBottom: 'var(--space-4)' }}>
          <h3>Filters</h3>
          <label>
            Scope <input value={scopeFilter} onChange={(e) => setScopeFilter(e.target.value)} placeholder="e.g. global" />
          </label>
        </div>
      )}
      {isReadOnly && (
        <div className="card" style={{ marginBottom: 'var(--space-4)' }}>
          <h3>Snapshot Mode</h3>
          <p style={{ margin: 0, color: 'var(--color-text-muted)' }}>
            Policy data is snapshot-backed. Only published policy records included in snapshot are shown.
          </p>
        </div>
      )}
      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
      {loading ? <p style={{ color: 'var(--color-text-muted)' }}>Loading…</p> : <DataTable columns={columns} data={list} keyFn={(r) => `${r.scope_key}:${r.version}`} emptyMessage="No policy versions" />}
      {!isReadOnly && (
        <ActionPanel title="Publish policy" actions={[{ label: 'Publish new version', onClick: handlePublish, variant: 'primary' }]} />
      )}
    </div>
  );
}
