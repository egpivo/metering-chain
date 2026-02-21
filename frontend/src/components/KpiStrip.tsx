import { formatInteger } from '../utils/format';
import type { DemoWindowAggregate } from '../domain/types';

interface KpiStripProps {
  windows: DemoWindowAggregate[];
}

export function KpiStrip({ windows }: KpiStripProps) {
  const totalGross = windows.reduce((s, w) => s + w.gross_spent, 0);
  const totalOperatorShare = windows.reduce((s, w) => s + w.operator_share, 0);
  const disputedCount = windows.filter((w) => w.status?.toLowerCase().includes('disputed')).length;
  const avgTopN = windows.length ? windows.reduce((s, w) => s + w.top_n_share, 0) / windows.length : 0;
  const totalOperators = Math.max(0, ...windows.map((w) => w.operator_count));

  return (
    <div
      className="kpi-strip"
      style={{
        display: 'flex',
        flexWrap: 'wrap',
        gap: 'var(--space-6)',
        padding: 'var(--space-3) 0',
        marginBottom: 'var(--space-4)',
        borderBottom: '1px solid var(--color-border)',
      }}
    >
      <div>
        <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Windows </span>
        <strong>{windows.length}</strong>
      </div>
      <div>
        <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Total gross </span>
        <strong className="mono">{formatInteger(totalGross)}</strong>
      </div>
      <div>
        <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Operator share (sum) </span>
        <strong className="mono">{formatInteger(totalOperatorShare)}</strong>
      </div>
      <div>
        <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Disputed </span>
        <strong style={{ color: disputedCount > 0 ? 'var(--color-demo-mismatch)' : undefined }}>{disputedCount}</strong>
      </div>
      <div>
        <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Avg top-N share </span>
        <strong>{avgTopN.toFixed(1)}%</strong>
      </div>
      <div>
        <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Max operators (window) </span>
        <strong>{formatInteger(totalOperators)}</strong>
      </div>
    </div>
  );
}
