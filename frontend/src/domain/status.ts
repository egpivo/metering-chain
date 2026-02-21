/**
 * Deterministic status labels for UI (backend enum → display).
 */

export function settlementStatusLabel(status: string): string {
  const s = (status || '').toLowerCase();
  if (s.includes('proposed')) return 'Proposed';
  if (s.includes('finalized')) return 'Finalized';
  if (s.includes('disputed')) return 'Disputed';
  return status || '—';
}

export function claimStatusLabel(status: string): string {
  const s = (status || '').toLowerCase();
  if (s.includes('pending')) return 'Pending';
  if (s.includes('paid')) return 'Paid';
  if (s.includes('rejected')) return 'Rejected';
  return status || '—';
}

export function disputeStatusLabel(status: string): string {
  const s = (status || '').toLowerCase();
  if (s.includes('open')) return 'Open';
  if (s.includes('upheld')) return 'Upheld';
  if (s.includes('dismissed')) return 'Dismissed';
  return status || '—';
}

export function policyStatusLabel(status: string): string {
  const s = (status || '').toLowerCase();
  if (s.includes('published')) return 'Published';
  if (s.includes('superseded')) return 'Superseded';
  return status || '—';
}
