/**
 * Backend error_code → ApiErrorView (suggested_action from docs/error_codes.md).
 */

import type { ApiErrorView } from './types';

const SUGGESTED_ACTIONS: Record<string, string> = {
  DUPLICATE_SETTLEMENT_WINDOW: 'Use a different window_id for this owner/service',
  SETTLEMENT_NOT_FOUND: 'Propose settlement first',
  SETTLEMENT_NOT_PROPOSED: 'Settlement must be in Proposed state',
  SETTLEMENT_NOT_FINALIZED: 'Finalize settlement before claim',
  CLAIM_AMOUNT_EXCEEDS_PAYABLE: 'claim_amount ≤ remaining payable',
  CLAIM_NOT_PENDING: 'Claim already paid or rejected',
  SETTLEMENT_CONSERVATION_VIOLATION: 'gross_spent = operator_share + protocol_fee + reserve_locked',
  DISPUTE_ALREADY_OPEN: 'Resolve existing dispute first',
  DISPUTE_NOT_FOUND: 'Invalid dispute id',
  DISPUTE_NOT_OPEN: 'Target dispute must be Open',
  INVALID_POLICY_PARAMETERS: 'operator_share_bps + protocol_fee_bps = 10000; dispute_window_secs > 0',
  POLICY_VERSION_CONFLICT: 'Duplicate (scope, version) or non-monotonic version',
  POLICY_NOT_FOUND: 'No policy for scope (e.g. Supersede target)',
  POLICY_NOT_EFFECTIVE: 'effective_from_tx_id > current_tx_id',
  RETROACTIVE_POLICY_FORBIDDEN: 'effective_from_tx_id must be >= next_tx_id',
  INVALID_EVIDENCE_BUNDLE: 'Evidence bundle shape invalid or replay_hash empty',
  REPLAY_MISMATCH: 'Replay result does not match settlement totals or replay_hash',
  EVIDENCE_NOT_FOUND: 'Evidence or bundle not found',
  INVALID_TRANSACTION: 'Check payload and preconditions',
  SIGNATURE_VERIFICATION: 'Sign with correct wallet',
};

export function toApiErrorView(error_code: string, message?: string): ApiErrorView {
  const code = (error_code || 'UNKNOWN').toUpperCase().replace(/\s+/g, '_');
  return {
    error_code: code,
    message: message || code,
    suggested_action: SUGGESTED_ACTIONS[code] ?? 'See error_code and retry.',
  };
}

export function parseBackendError(body: unknown): ApiErrorView {
  if (body && typeof body === 'object' && 'error_code' in body) {
    const o = body as { error_code?: string; message?: string };
    return toApiErrorView(o.error_code ?? 'UNKNOWN', o.message as string | undefined);
  }
  if (typeof body === 'string') return parseCliStderr(body);
  return toApiErrorView('UNKNOWN', 'Unknown error');
}

/** Map CLI stderr "Error: ..." text to error_code (backend uses Display, not error_code). */
const MESSAGE_TO_CODE: [RegExp, string][] = [
  [/duplicate settlement window/i, 'DUPLICATE_SETTLEMENT_WINDOW'],
  [/settlement not found/i, 'SETTLEMENT_NOT_FOUND'],
  [/not proposed|already finalized/i, 'SETTLEMENT_NOT_PROPOSED'],
  [/settlement not finalized/i, 'SETTLEMENT_NOT_FINALIZED'],
  [/claim amount exceeds payable/i, 'CLAIM_AMOUNT_EXCEEDS_PAYABLE'],
  [/claim not found|not pending/i, 'CLAIM_NOT_PENDING'],
  [/conservation violation|gross_spent/i, 'SETTLEMENT_CONSERVATION_VIOLATION'],
  [/dispute already open/i, 'DISPUTE_ALREADY_OPEN'],
  [/dispute not found/i, 'DISPUTE_NOT_FOUND'],
  [/dispute not open/i, 'DISPUTE_NOT_OPEN'],
  [/invalid policy parameters|bps sum/i, 'INVALID_POLICY_PARAMETERS'],
  [/policy version conflict/i, 'POLICY_VERSION_CONFLICT'],
  [/policy not found/i, 'POLICY_NOT_FOUND'],
  [/policy not effective/i, 'POLICY_NOT_EFFECTIVE'],
  [/retroactive policy forbidden/i, 'RETROACTIVE_POLICY_FORBIDDEN'],
  [/invalid evidence bundle/i, 'INVALID_EVIDENCE_BUNDLE'],
  [/replay.*mismatch|does not match settlement/i, 'REPLAY_MISMATCH'],
  [/evidence.*not found|bundle not found/i, 'EVIDENCE_NOT_FOUND'],
  [/signature verification/i, 'SIGNATURE_VERIFICATION'],
  [/invalid transaction/i, 'INVALID_TRANSACTION'],
];

export function parseCliStderr(stderr: string): ApiErrorView {
  const msg = (stderr || '').replace(/^Error:\s*/i, '').trim();
  for (const [re, code] of MESSAGE_TO_CODE) {
    if (re.test(msg)) return toApiErrorView(code, msg);
  }
  return toApiErrorView('UNKNOWN', msg || 'CLI error');
}
