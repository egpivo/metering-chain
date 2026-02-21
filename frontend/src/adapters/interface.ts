import type {
  SettlementView,
  ClaimView,
  DisputeView,
  PolicyVersionView,
  EvidenceBundleView,
  ApiErrorView,
  ListSettlementsFilters,
  ListPolicyFilters,
} from '../domain/types';

export interface FrontendDataAdapter {
  /** When true, UI should hide/disable write actions (snapshot/demo mode). */
  readonlyMode?: boolean;
  listSettlements(filters?: ListSettlementsFilters): Promise<SettlementView[]>;
  getSettlement(owner: string, serviceId: string, windowId: string): Promise<SettlementView | null>;
  listClaims(owner?: string, serviceId?: string, windowId?: string): Promise<ClaimView[]>;
  getDispute(owner: string, serviceId: string, windowId: string): Promise<DisputeView | null>;
  getEvidenceBundle(owner: string, serviceId: string, windowId: string): Promise<EvidenceBundleView | ApiErrorView | null>;
  listPolicies(filters?: ListPolicyFilters): Promise<PolicyVersionView[]>;
  getPolicy(scopeKey: string, version: number): Promise<PolicyVersionView | null>;

  finalizeSettlement(owner: string, serviceId: string, windowId: string): Promise<{ ok: true } | ApiErrorView>;
  submitClaim(operator: string, owner: string, serviceId: string, windowId: string, amount: number): Promise<{ ok: true } | ApiErrorView>;
  payClaim(operator: string, owner: string, serviceId: string, windowId: string): Promise<{ ok: true } | ApiErrorView>;
  openDispute(owner: string, serviceId: string, windowId: string, reasonCode?: string): Promise<{ ok: true } | ApiErrorView>;
  resolveDispute(owner: string, serviceId: string, windowId: string, verdict: 'upheld' | 'dismissed'): Promise<{ ok: true } | ApiErrorView>;
  publishPolicy(params: {
    scope: string;
    version: number;
    effective_from_tx_id: number;
    operator_share_bps: number;
    protocol_fee_bps: number;
    dispute_window_secs: number;
    reserve_fixed?: number;
  }): Promise<{ ok: true } | ApiErrorView>;
}
