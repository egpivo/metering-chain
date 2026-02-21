import { settlementStatusLabel, claimStatusLabel, disputeStatusLabel, policyStatusLabel } from '../domain/status';

type Kind = 'settlement' | 'claim' | 'dispute' | 'policy';

function labelFor(kind: Kind, status: string): string {
  switch (kind) {
    case 'settlement': return settlementStatusLabel(status);
    case 'claim': return claimStatusLabel(status);
    case 'dispute': return disputeStatusLabel(status);
    case 'policy': return policyStatusLabel(status);
    default: return status || '—';
  }
}

function badgeClass(status: string): string {
  const s = (status || '').toLowerCase();
  if (s.includes('proposed') || s.includes('pending')) return 'badge badge--proposed';
  if (s.includes('finalized') || s.includes('paid') || s.includes('published')) return 'badge badge--finalized';
  if (s.includes('disputed') || s.includes('open')) return 'badge badge--disputed';
  if (s.includes('upheld') || s.includes('dismissed') || s.includes('superseded')) return 'badge badge--dismissed';
  return 'badge';
}

interface StatusBadgeProps {
  kind: Kind;
  status: string;
}

export function StatusBadge({ kind, status }: StatusBadgeProps) {
  const label = labelFor(kind, status);
  return <span className={badgeClass(status)}>{label}</span>;
}
