import { formatInteger } from '../utils/format';
import type { EvidenceBundleView } from '../domain/types';

interface EvidenceCompareCardProps {
  bundle: EvidenceBundleView;
  /** Recorded totals from settlement (for comparison) */
  recorded?: { gross_spent: number; operator_share: number; protocol_fee: number; reserve_locked: number };
  mismatch?: boolean;
  /** Optional class for embedding in narrow panels (e.g. demo Block 4) */
  className?: string;
}

export function EvidenceCompareCard({ bundle, recorded, mismatch, className }: EvidenceCompareCardProps) {
  const r = bundle.replay_summary;
  return (
    <div className={className ? `card ${className}` : 'card'}>
      <h3>Evidence & Replay (G4)</h3>
      <dl style={{ margin: 0, display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 'var(--space-2) var(--space-4)' }}>
        <dt style={{ color: 'var(--color-text-muted)' }}>Settlement</dt>
        <dd className="mono">{bundle.settlement_key}</dd>
        <dt style={{ color: 'var(--color-text-muted)' }}>Tx range</dt>
        <dd className="mono">{bundle.from_tx_id} … {bundle.to_tx_id}</dd>
        <dt style={{ color: 'var(--color-text-muted)' }}>Evidence hash</dt>
        <dd className="mono">{bundle.evidence_hash}</dd>
        <dt style={{ color: 'var(--color-text-muted)' }}>Replay hash</dt>
        <dd className="mono">{bundle.replay_hash}</dd>
      </dl>
      {r && (
        <>
          <h4 style={{ margin: 'var(--space-4) 0 var(--space-2)', fontSize: 'var(--text-sm)' }}>Replay summary</h4>
          <table className="data-table evidence-compare-table">
            <tbody>
              <CompareRow label="gross_spent" replay={r.gross_spent} recorded={recorded?.gross_spent} mismatch={mismatch} />
              <CompareRow label="operator_share" replay={r.operator_share} recorded={recorded?.operator_share} mismatch={mismatch} />
              <CompareRow label="protocol_fee" replay={r.protocol_fee} recorded={recorded?.protocol_fee} mismatch={mismatch} />
              <CompareRow label="reserve_locked" replay={r.reserve_locked} recorded={recorded?.reserve_locked} mismatch={mismatch} />
            </tbody>
          </table>
          {mismatch && (
            <p style={{ color: 'var(--color-danger)', marginTop: 'var(--space-2)' }}>
              Replay result does not match settlement totals. Resolve is blocked until evidence matches.
            </p>
          )}
        </>
      )}
    </div>
  );
}

function CompareRow({
  label,
  replay,
  recorded,
  mismatch,
}: { label: string; replay: number; recorded?: number; mismatch?: boolean }) {
  const same = recorded === undefined || recorded === replay;
  return (
    <tr style={mismatch && !same ? { background: 'rgba(207, 34, 46, 0.1)' } : undefined}>
      <td style={{ color: 'var(--color-text-muted)' }}>{label}</td>
      <td className="mono">{formatInteger(replay)}</td>
      {recorded !== undefined && (
        <td className="mono">{same ? '✓' : `${formatInteger(recorded)} (recorded)`}</td>
      )}
    </tr>
  );
}
