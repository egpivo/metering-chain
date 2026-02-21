import type { DemoWindowAggregate, DemoEvidenceView } from '../domain/types';

/**
 * Demo analytics adapter (phase4_demo_data_plan).
 * Separate from FrontendDataAdapter; used by /demo/phase4 for snapshot or BYOK data.
 */
export interface DemoAnalyticsAdapter {
  getDemoWindows(params: {
    start_date: string;
    end_date: string;
    owner?: string;
    service_id?: string;
    window_granularity: 'day' | 'week';
    operator_share_bps?: number;
    protocol_fee_bps?: number;
    reserve_bps?: number;
    dispute_window_secs?: number;
    top_n?: number;
  }): Promise<DemoWindowAggregate[]>;

  getDemoEvidence(
    window_id: string,
    owner: string,
    service_id: string
  ): Promise<DemoEvidenceView | null>;
}
