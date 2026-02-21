import type { FrontendDataAdapter } from './interface';
import type {
  ApiErrorView,
  ClaimView,
  DisputeView,
  EvidenceBundleView,
  ListPolicyFilters,
  ListSettlementsFilters,
  PolicyVersionView,
  SettlementView,
} from '../domain/types';

const SNAPSHOT_URL = `${import.meta.env.BASE_URL ?? '/'}demo_data/phase4_snapshot.json`;

interface SnapshotWindowRaw {
  owner: string;
  service_id: string;
  window_id: string;
  gross_spent: number;
  operator_share: number;
  protocol_fee: number;
  reserve_locked: number;
  status?: string;
  evidence_hash?: string;
  replay_hash?: string | null;
  replay_summary?: SettlementView['replay_summary'];
  from_tx_id?: number;
  to_tx_id?: number;
}

interface SnapshotPayload {
  windows: SnapshotWindowRaw[];
}

let cached: SnapshotPayload | null = null;

async function loadSnapshot(): Promise<SnapshotPayload> {
  if (cached) return cached;
  const res = await fetch(SNAPSHOT_URL);
  if (!res.ok) throw new Error(`snapshot load failed: ${res.status}`);
  const data = (await res.json()) as SnapshotPayload;
  if (!data || !Array.isArray(data.windows)) {
    throw new Error('invalid snapshot payload');
  }
  cached = data;
  return data;
}

function toSettlement(w: SnapshotWindowRaw): SettlementView {
  const settlementId = `${w.owner}:${w.service_id}:${w.window_id}`;
  const payable = Math.max(0, w.operator_share);
  return {
    settlement_id: settlementId,
    owner: w.owner,
    service_id: w.service_id,
    window_id: w.window_id,
    status: w.status ?? 'Proposed',
    gross_spent: w.gross_spent,
    operator_share: w.operator_share,
    protocol_fee: w.protocol_fee,
    reserve_locked: w.reserve_locked,
    payable,
    total_paid: 0,
    evidence_hash: w.evidence_hash ?? '',
    from_tx_id: w.from_tx_id ?? 0,
    to_tx_id: w.to_tx_id ?? 0,
    replay_hash: w.replay_hash ?? null,
    replay_summary: w.replay_summary ?? null,
    claims: [],
  };
}

function filterSettlements(list: SettlementView[], f?: ListSettlementsFilters): SettlementView[] {
  if (!f) return list;
  return list.filter((s) => {
    if (f.owner && s.owner !== f.owner) return false;
    if (f.service_id && s.service_id !== f.service_id) return false;
    if (f.status && !s.status.toLowerCase().includes(f.status.toLowerCase())) return false;
    if (f.start_date && s.window_id < f.start_date) return false;
    if (f.end_date && s.window_id > f.end_date) return false;
    return true;
  });
}

function readonlyAction(): ApiErrorView {
  return {
    error_code: 'DEMO_READ_ONLY',
    message: 'This screen is backed by snapshot data; write actions are disabled.',
    suggested_action: 'Use CLI/API flow for write operations or switch to live backend adapter.',
  };
}

const DEFAULT_POLICIES: PolicyVersionView[] = [
  {
    scope_key: 'global',
    version: 1,
    effective_from_tx_id: 0,
    status: 'Published',
    operator_share_bps: 9000,
    protocol_fee_bps: 1000,
    dispute_window_secs: 86400,
  },
];

export const SnapshotFrontendAdapter: FrontendDataAdapter = {
  readonlyMode: true,
  async listSettlements(filters) {
    const payload = await loadSnapshot();
    const list = payload.windows.map(toSettlement);
    return filterSettlements(list, filters);
  },
  async getSettlement(owner, serviceId, windowId) {
    const payload = await loadSnapshot();
    const w = payload.windows.find(
      (x) => x.owner === owner && x.service_id === serviceId && x.window_id === windowId
    );
    return w ? toSettlement(w) : null;
  },
  async listClaims(_owner?: string, _serviceId?: string, _windowId?: string) {
    return [] as ClaimView[];
  },
  async getDispute(owner, serviceId, windowId) {
    const settlement = await this.getSettlement(owner, serviceId, windowId);
    if (!settlement || !settlement.status.toLowerCase().includes('disputed')) return null;
    const d: DisputeView = {
      settlement_key: settlement.settlement_id,
      status: 'Open',
      resolution_audit: settlement.replay_hash && settlement.replay_summary
        ? { replay_hash: settlement.replay_hash, replay_summary: settlement.replay_summary }
        : null,
    };
    return d;
  },
  async getEvidenceBundle(owner, serviceId, windowId) {
    const settlement = await this.getSettlement(owner, serviceId, windowId);
    if (!settlement || !settlement.replay_hash || !settlement.replay_summary) return null;
    const e: EvidenceBundleView = {
      settlement_key: settlement.settlement_id,
      from_tx_id: settlement.from_tx_id,
      to_tx_id: settlement.to_tx_id,
      evidence_hash: settlement.evidence_hash,
      replay_hash: settlement.replay_hash,
      replay_summary: settlement.replay_summary,
    };
    return e;
  },
  async listPolicies(filters?: ListPolicyFilters) {
    if (!filters?.scope) return DEFAULT_POLICIES;
    return DEFAULT_POLICIES.filter((p) => p.scope_key.includes(filters.scope!));
  },
  async getPolicy(scopeKey, version) {
    return DEFAULT_POLICIES.find((p) => p.scope_key === scopeKey && p.version === version) ?? null;
  },
  async finalizeSettlement() {
    return readonlyAction();
  },
  async submitClaim() {
    return readonlyAction();
  },
  async payClaim() {
    return readonlyAction();
  },
  async openDispute() {
    return readonlyAction();
  },
  async resolveDispute() {
    return readonlyAction();
  },
  async publishPolicy() {
    return readonlyAction();
  },
};
