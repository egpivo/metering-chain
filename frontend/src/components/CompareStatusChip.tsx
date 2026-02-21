import type { DemoCompareStatus } from '../domain/types';

interface CompareStatusChipProps {
  status: DemoCompareStatus;
}

const styles: Record<DemoCompareStatus, React.CSSProperties> = {
  MATCH: { background: 'var(--color-demo-match)', color: '#fff', padding: 'var(--space-1) var(--space-2)', borderRadius: 'var(--radius)', fontWeight: 600, fontSize: 'var(--text-xs)' },
  MISMATCH: { background: 'var(--color-demo-mismatch)', color: '#fff', padding: 'var(--space-1) var(--space-2)', borderRadius: 'var(--radius)', fontWeight: 600, fontSize: 'var(--text-xs)' },
  MISSING: { background: 'var(--color-demo-frozen)', color: '#fff', padding: 'var(--space-1) var(--space-2)', borderRadius: 'var(--radius)', fontWeight: 600, fontSize: 'var(--text-xs)' },
};

export function CompareStatusChip({ status }: CompareStatusChipProps) {
  return <span style={styles[status]}>{status}</span>;
}
