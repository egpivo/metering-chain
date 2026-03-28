import type { FrontendDataAdapter } from './interface';
import { MockAdapter } from './mock-adapter';

/**
 * Blog/screenshot wiring: disputed list succeeds, per-settlement detail fetch fails.
 * Route: `/demo/screenshot/disputes-detail-error` (nested AdapterProvider in App).
 */
export const ScreenshotDisputesDetailErrorAdapter: FrontendDataAdapter = {
  ...MockAdapter,
  async listSettlements(filters) {
    if (filters?.status === 'disputed') {
      return [
        {
          settlement_id: 'alice:storage:w1',
          owner: 'alice',
          service_id: 'storage',
          window_id: 'w1',
          status: 'Disputed',
          gross_spent: 50,
          operator_share: 45,
          protocol_fee: 5,
          reserve_locked: 0,
          payable: 45,
          total_paid: 0,
          evidence_hash: 'eh',
          from_tx_id: 0,
          to_tx_id: 3,
          replay_hash: null,
          replay_summary: null,
        },
      ];
    }
    return MockAdapter.listSettlements(filters);
  },
  async getDispute() {
    throw {
      error_code: 'DISPUTE_LOOKUP_FAILED',
      message: 'detail query failed',
      suggested_action: 'retry dispute query',
    };
  },
};
