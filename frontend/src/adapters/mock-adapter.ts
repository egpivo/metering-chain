import type { FrontendDataAdapter } from './interface';
import type {
  SettlementView,
  ClaimView,
  DisputeView,
  PolicyVersionView,
  EvidenceBundleView,
  ListSettlementsFilters,
  ListPolicyFilters,
} from '../domain/types';

const MOCK_SETTLEMENTS: SettlementView[] = [
  {
    settlement_id: 'alice:storage:w1',
    owner: 'alice',
    service_id: 'storage',
    window_id: 'w1',
    status: 'Finalized',
    gross_spent: 50,
    operator_share: 45,
    protocol_fee: 5,
    reserve_locked: 0,
    payable: 45,
    total_paid: 0,
    evidence_hash: 'a1b2c3',
    from_tx_id: 0,
    to_tx_id: 3,
    replay_hash: 'r1',
    replay_summary: { from_tx_id: 0, to_tx_id: 3, tx_count: 3, gross_spent: 50, operator_share: 45, protocol_fee: 5, reserve_locked: 0 },
  },
  {
    settlement_id: 'bob:compute:w1',
    owner: 'bob',
    service_id: 'compute',
    window_id: 'w1',
    status: 'Proposed',
    gross_spent: 120,
    operator_share: 108,
    protocol_fee: 12,
    reserve_locked: 0,
    payable: 108,
    total_paid: 0,
    evidence_hash: 'd4e5f6',
    from_tx_id: 2,
    to_tx_id: 5,
    replay_hash: null,
    replay_summary: null,
  },
];

const MOCK_CLAIMS: ClaimView[] = [
  { claim_id: 'alice:alice:storage:w1', operator: 'alice', claim_amount: 45, status: 'Pending', settlement_key: 'alice:storage:w1' },
];

const MOCK_DISPUTES: { key: string; value: DisputeView }[] = [];

const MOCK_POLICIES: PolicyVersionView[] = [
  { scope_key: 'global', version: 1, effective_from_tx_id: 0, status: 'Published', operator_share_bps: 9000, protocol_fee_bps: 1000, dispute_window_secs: 86400 },
];

function filterSettlements(list: SettlementView[], f?: ListSettlementsFilters): SettlementView[] {
  if (!f) return list;
  return list.filter((s) => {
    if (f.owner && s.owner !== f.owner) return false;
    if (f.service_id && s.service_id !== f.service_id) return false;
    if (f.status && !s.status.toLowerCase().includes(f.status.toLowerCase())) return false;
    return true;
  });
}

function filterPolicies(list: PolicyVersionView[], f?: ListPolicyFilters): PolicyVersionView[] {
  if (!f?.scope) return list;
  return list.filter((p) => p.scope_key.includes(f.scope!));
}

export const MockAdapter: FrontendDataAdapter = {
  readonlyMode: false,
  async listSettlements(filters) {
    return Promise.resolve(filterSettlements([...MOCK_SETTLEMENTS], filters));
  },
  async getSettlement(owner, serviceId, windowId) {
    const s = MOCK_SETTLEMENTS.find(
      (x) => x.owner === owner && x.service_id === serviceId && x.window_id === windowId
    );
    return Promise.resolve(s ? { ...s } : null);
  },
  async listClaims(owner, serviceId, windowId) {
    let list = [...MOCK_CLAIMS];
    if (owner) list = list.filter((c) => (c as ClaimView & { settlement_key?: string }).settlement_key?.startsWith(owner));
    if (serviceId) list = list.filter((c) => (c as ClaimView & { settlement_key?: string }).settlement_key?.includes(serviceId));
    if (windowId) list = list.filter((c) => (c as ClaimView & { settlement_key?: string }).settlement_key?.endsWith(windowId));
    return Promise.resolve(list);
  },
  async getDispute(owner, serviceId, windowId) {
    const key = `${owner}:${serviceId}:${windowId}`;
    const d = MOCK_DISPUTES.find((x) => x.key === key);
    return Promise.resolve(d ? d.value : null);
  },
  async getEvidenceBundle(owner, serviceId, windowId) {
    const s = MOCK_SETTLEMENTS.find(
      (x) => x.owner === owner && x.service_id === serviceId && x.window_id === windowId
    );
    if (!s || !s.replay_hash || !s.replay_summary) return Promise.resolve(null);
    const bundle: EvidenceBundleView = {
      settlement_key: s.settlement_id,
      from_tx_id: s.from_tx_id,
      to_tx_id: s.to_tx_id,
      evidence_hash: s.evidence_hash,
      replay_hash: s.replay_hash,
      replay_summary: s.replay_summary,
    };
    return Promise.resolve(bundle);
  },
  async listPolicies(filters) {
    return Promise.resolve(filterPolicies([...MOCK_POLICIES], filters));
  },
  async getPolicy(scopeKey, version) {
    const p = MOCK_POLICIES.find((x) => x.scope_key === scopeKey && x.version === version);
    return Promise.resolve(p ? { ...p } : null);
  },
  async finalizeSettlement() {
    return Promise.resolve({ ok: true as const });
  },
  async submitClaim() {
    return Promise.resolve({ ok: true as const });
  },
  async payClaim() {
    return Promise.resolve({ ok: true as const });
  },
  async openDispute() {
    return Promise.resolve({ ok: true as const });
  },
  async resolveDispute() {
    return Promise.resolve({ ok: true as const });
  },
  async publishPolicy() {
    return Promise.resolve({ ok: true as const });
  },
};
