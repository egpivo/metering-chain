import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useAdapter } from '../adapters/context';
import { StatusBadge } from '../components/StatusBadge';
import { Timeline } from '../components/Timeline';
import { ErrorBanner } from '../components/ErrorBanner';
import { ActionPanel } from '../components/ActionPanel';
import { EvidenceCompareCard } from '../components/EvidenceCompareCard';
import type { SettlementView, ApiErrorView, EvidenceBundleView } from '../domain/types';

export function SettlementDetailPage() {
  const { owner, serviceId, windowId } = useParams<{ owner: string; serviceId: string; windowId: string }>();
  const navigate = useNavigate();
  const adapter = useAdapter();
  const [settlement, setSettlement] = useState<SettlementView | null>(null);
  const [evidence, setEvidence] = useState<EvidenceBundleView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ApiErrorView | null>(null);
  const [actionError, setActionError] = useState<ApiErrorView | null>(null);

  useEffect(() => {
    if (!owner || !serviceId || !windowId) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    adapter
      .getSettlement(owner, serviceId, windowId)
      .then((s) => {
        if (!cancelled) setSettlement(s ?? null);
      })
      .catch((e: unknown) => {
        if (!cancelled && e && typeof e === 'object' && 'error_code' in e) setError(e as ApiErrorView);
        else setError({ error_code: 'UNKNOWN', message: String(e), suggested_action: 'Retry.' });
      })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [adapter, owner, serviceId, windowId]);

  useEffect(() => {
    if (!owner || !serviceId || !windowId) return;
    adapter.getEvidenceBundle(owner, serviceId, windowId).then((b) => {
      if (b && !('error_code' in b)) setEvidence(b);
      else setEvidence(null);
    }).catch(() => setEvidence(null));
  }, [adapter, owner, serviceId, windowId]);

  if (!owner || !serviceId || !windowId) {
    return <p>Missing owner/serviceId/windowId</p>;
  }

  const handleFinalize = () => {
    setActionError(null);
    adapter.finalizeSettlement(owner, serviceId, windowId).then((r) => {
      if ('ok' in r && r.ok) {
        adapter.getSettlement(owner, serviceId, windowId).then(setSettlement);
      } else setActionError(r as ApiErrorView);
    }).catch((e: unknown) => setActionError(e && typeof e === 'object' && 'error_code' in e ? e as ApiErrorView : { error_code: 'UNKNOWN', message: String(e), suggested_action: '' }));
  };

  const handleOpenDispute = () => {
    setActionError(null);
    adapter.openDispute(owner, serviceId, windowId).then((r) => {
      if ('ok' in r && r.ok) adapter.getSettlement(owner, serviceId, windowId).then(setSettlement);
      else setActionError(r as ApiErrorView);
    }).catch((e: unknown) => setActionError(e && typeof e === 'object' && 'error_code' in e ? e as ApiErrorView : { error_code: 'UNKNOWN', message: String(e), suggested_action: '' }));
  };

  const status = settlement?.status?.toLowerCase() ?? '';
  const isProposed = status.includes('proposed');
  const isFinalized = status.includes('finalized');
  const isDisputed = status.includes('disputed');
  const steps = [
    { label: 'Proposed', done: isProposed || isFinalized || isDisputed, current: isProposed },
    { label: 'Finalized', done: isFinalized || isDisputed, current: isFinalized && !isDisputed },
    { label: 'Claimed / Disputed', done: isDisputed, current: isDisputed },
    { label: 'Resolved', done: false, current: false },
  ];

  if (loading) return <p>Loading settlement…</p>;
  if (error) return <ErrorBanner error={error} />;
  if (!settlement) return <p>Settlement not found.</p>;

  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-2)' }}>Settlement: {settlement.settlement_id}</h1>
      <p style={{ color: 'var(--color-text-muted)', marginBottom: 'var(--space-4)' }}>
        <StatusBadge kind="settlement" status={settlement.status} />
      </p>
      {actionError && <ErrorBanner error={actionError} onDismiss={() => setActionError(null)} />}

      <div className="card">
        <h3>Status</h3>
        <Timeline steps={steps} />
      </div>
      <div className="card">
        <h3>Economics</h3>
        <dl style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 'var(--space-2) var(--space-4)' }}>
          <dt style={{ color: 'var(--color-text-muted)' }}>Gross spent</dt><dd>{settlement.gross_spent}</dd>
          <dt style={{ color: 'var(--color-text-muted)' }}>Operator share</dt><dd>{settlement.operator_share}</dd>
          <dt style={{ color: 'var(--color-text-muted)' }}>Protocol fee</dt><dd>{settlement.protocol_fee}</dd>
          <dt style={{ color: 'var(--color-text-muted)' }}>Reserve locked</dt><dd>{settlement.reserve_locked}</dd>
          <dt style={{ color: 'var(--color-text-muted)' }}>Payable</dt><dd>{settlement.payable}</dd>
          <dt style={{ color: 'var(--color-text-muted)' }}>Total paid</dt><dd>{settlement.total_paid}</dd>
        </dl>
      </div>
      <div className="card">
        <h3>Integrity</h3>
        <dl style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 'var(--space-2) var(--space-4)' }}>
          <dt style={{ color: 'var(--color-text-muted)' }}>Evidence hash</dt><dd className="mono">{settlement.evidence_hash}</dd>
          <dt style={{ color: 'var(--color-text-muted)' }}>Replay hash</dt><dd className="mono">{settlement.replay_hash ?? '—'}</dd>
          <dt style={{ color: 'var(--color-text-muted)' }}>Tx range</dt><dd className="mono">{settlement.from_tx_id} … {settlement.to_tx_id}</dd>
        </dl>
      </div>
      {evidence && (
        <EvidenceCompareCard
          bundle={evidence}
          recorded={{
            gross_spent: settlement.gross_spent,
            operator_share: settlement.operator_share,
            protocol_fee: settlement.protocol_fee,
            reserve_locked: settlement.reserve_locked,
          }}
        />
      )}
      <ActionPanel
        title="Actions"
        actions={[
          { label: 'Finalize Settlement', onClick: handleFinalize, variant: 'primary', disabled: !isProposed },
          { label: 'Open Dispute', onClick: handleOpenDispute, variant: 'danger', disabled: !isFinalized },
          { label: 'Back to list', onClick: () => navigate('/settlements') },
        ]}
      />
    </div>
  );
}
