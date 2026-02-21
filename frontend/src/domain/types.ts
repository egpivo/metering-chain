/**
 * Frontend domain types — aligned with backend CLI JSON and Phase 4 spec.
 */

export interface SettlementView {
  settlement_id: string;
  owner: string;
  service_id: string;
  window_id: string;
  status: string;
  gross_spent: number;
  operator_share: number;
  protocol_fee: number;
  reserve_locked: number;
  payable: number;
  total_paid: number;
  evidence_hash: string;
  from_tx_id: number;
  to_tx_id: number;
  replay_hash?: string | null;
  replay_summary?: ReplaySummaryView | null;
  claims?: ClaimView[];
}

export interface ReplaySummaryView {
  from_tx_id: number;
  to_tx_id: number;
  tx_count: number;
  gross_spent: number;
  operator_share: number;
  protocol_fee: number;
  reserve_locked: number;
}

export interface ClaimView {
  claim_id: string;
  operator: string;
  claim_amount: number;
  status: string;
  settlement_key?: string;
}

export interface DisputeView {
  settlement_key: string;
  status: string;
  resolution_audit?: {
    replay_hash: string;
    replay_summary: ReplaySummaryView;
  } | null;
}

export interface PolicyVersionView {
  scope_key: string;
  version: number;
  effective_from_tx_id: number;
  status: string;
  published_by?: string;
  operator_share_bps?: number;
  protocol_fee_bps?: number;
  dispute_window_secs?: number;
}

export interface EvidenceBundleView {
  settlement_key: string;
  from_tx_id: number;
  to_tx_id: number;
  evidence_hash: string;
  replay_hash: string;
  replay_summary: ReplaySummaryView;
}

export interface ApiErrorView {
  error_code: string;
  message: string;
  suggested_action: string;
}

export type ListSettlementsFilters = {
  owner?: string;
  service_id?: string;
  status?: string;
};

export type ListPolicyFilters = { scope?: string };

// --- Demo analytics (phase4_demo_data_plan) ---

export interface DemoUsageRow {
  ts: string;
  owner: string;
  service_id: string;
  operator: string;
  units: number;
  cost: number;
  tx_ref?: string;
}

export interface DemoWindowAggregate {
  owner: string;
  service_id: string;
  window_id: string;
  from_ts: string;
  to_ts: string;
  gross_spent: number;
  operator_share: number;
  protocol_fee: number;
  reserve_locked: number;
  top_n_share: number;
  operator_count: number;
  status?: string;
  evidence_hash?: string;
  replay_hash?: string | null;
  replay_summary?: ReplaySummaryView | null;
  from_tx_id?: number;
  to_tx_id?: number;
}

export interface DemoEvidenceView {
  window_id: string;
  evidence_hash: string;
  replay_hash: string;
  replay_summary: ReplaySummaryView;
}

/** Demo UI state (phase4_demo_ui_flow) */
export interface DemoUiState {
  mode: 'snapshot' | 'byok';
  loading: boolean;
  controls: {
    start_date: string;
    end_date: string;
    owner?: string;
    service_id?: string;
    window_granularity: 'day' | 'week';
    top_n: number;
    operator_share_bps: number;
    protocol_fee_bps: number;
    reserve_bps: number;
    dispute_window_secs: number;
  };
  selected_window_id?: string;
  last_error?: { error_code: string; message: string };
}

/** Compare status for evidence panel */
export type DemoCompareStatus = 'MATCH' | 'MISMATCH' | 'MISSING';

// --- Metering (phase4_metering_ui_reframe) ---

export interface MeteringSeriesPoint {
  ts: string;
  units: number;
  cost: number;
  owner_count?: number;
  window_count?: number;
}

export interface MeteringWindowPreview {
  window_id: string;
  from_ts: string;
  to_ts: string;
  usage_count: number;
  operator_count: number;
  gross_spent: number;
  owner?: string;
  service_id?: string;
}

export interface MeteringTopOperator {
  owner: string;
  service_id: string;
  units: number;
  cost: number;
  window_count: number;
}

export interface MeteringCounters {
  total_units: number;
  active_operators: number;
  windows_in_range: number;
  anomalies: number;
  /** Total cost (gross_spent) in range; used for "Total spent" in UI */
  total_cost?: number;
}

export interface MeteringAdapter {
  getMeteringSeries(params: { start_date: string; end_date: string; granularity: 'day' | 'week'; service_id?: string }): Promise<MeteringSeriesPoint[]>;
  getMeteringTopOperators(params: { start_date: string; end_date: string; limit?: number; service_id?: string }): Promise<MeteringTopOperator[]>;
  getWindowPreview(params: { start_date: string; end_date: string; service_id?: string }): Promise<{ count: number; windows: MeteringWindowPreview[] }>;
  getMeteringCounters(params: { start_date: string; end_date: string; service_id?: string }): Promise<MeteringCounters>;
}
