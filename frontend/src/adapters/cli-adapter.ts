/**
 * CLI adapter: runs metering-chain binary and parses JSON stdout / stderr.
 * Intended for Node (BFF/Electron) or E2E; use MockAdapter in browser-only SPA.
 */

import type { FrontendDataAdapter } from './interface';
import type {
  SettlementView,
  ClaimView,
  DisputeView,
  ListSettlementsFilters,
  ListPolicyFilters,
} from '../domain/types';
import { parseCliStderr } from '../domain/error-mapping';

export interface CliAdapterConfig {
  /** Path to metering-chain binary (default: metering-chain from PATH or ../target/debug/metering-chain) */
  binary?: string;
  /** Data directory for state/tx log */
  dataDir?: string;
  /** Run CLI via this function (for testing or custom runner). If not set, uses spawnSync in Node. */
  run?: (args: string[], options?: { cwd?: string }) => Promise<{ stdout: string; stderr: string; exitCode: number }>;
}

function defaultRun(args: string[], options?: { cwd?: string }): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports -- Node-only path; require needed for conditional import
    const { spawnSync } = require('child_process') as typeof import('child_process');
    const bin = (globalThis as unknown as { __METERING_CHAIN_BIN?: string }).__METERING_CHAIN_BIN ?? 'metering-chain';
    const allArgs = ['--format', 'json', ...args];
    if (options?.cwd) allArgs.push('--data-dir', options.cwd);
    const r = spawnSync(bin, allArgs, {
      encoding: 'utf8',
      cwd: options?.cwd,
      env: { ...(typeof process !== 'undefined' ? process.env : {}), METERING_CHAIN_DATA_DIR: options?.cwd ?? '' },
    });
    return Promise.resolve({
      stdout: (r.stdout || '').trim(),
      stderr: (r.stderr || '').trim(),
      exitCode: r.status ?? (r.signal ? 1 : 0),
    });
  } catch {
    return Promise.resolve({ stdout: '', stderr: 'CLI not available (child_process)', exitCode: 1 });
  }
}

function mapListToSettlementView(o: {
  settlement_id: string;
  owner: string;
  service_id: string;
  window_id: string;
  status: string;
  gross_spent: number;
  operator_share: number;
  payable: number;
}): SettlementView {
  return {
    settlement_id: o.settlement_id,
    owner: o.owner,
    service_id: o.service_id,
    window_id: o.window_id,
    status: o.status,
    gross_spent: o.gross_spent,
    operator_share: o.operator_share,
    protocol_fee: 0,
    reserve_locked: 0,
    payable: o.payable,
    total_paid: 0,
    evidence_hash: '',
    from_tx_id: 0,
    to_tx_id: 0,
    replay_hash: null,
    replay_summary: null,
  };
}

function mapShowToSettlementView(o: {
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
  replay_summary?: ReplaySummaryLike | null;
  claims?: { operator: string; claim_amount: number; status: string }[];
}): SettlementView {
  const claims: ClaimView[] = (o.claims || []).map((c) => ({
    claim_id: `${c.operator}:${o.settlement_id}`,
    operator: c.operator,
    claim_amount: c.claim_amount,
    status: c.status,
    settlement_key: o.settlement_id,
  }));
  return {
    settlement_id: o.settlement_id,
    owner: o.owner,
    service_id: o.service_id,
    window_id: o.window_id,
    status: o.status,
    gross_spent: o.gross_spent,
    operator_share: o.operator_share,
    protocol_fee: o.protocol_fee,
    reserve_locked: o.reserve_locked,
    payable: o.payable,
    total_paid: o.total_paid,
    evidence_hash: o.evidence_hash,
    from_tx_id: o.from_tx_id,
    to_tx_id: o.to_tx_id,
    replay_hash: o.replay_hash ?? null,
    replay_summary: o.replay_summary ? mapReplaySummary(o.replay_summary) : null,
    claims,
  };
}

interface ReplaySummaryLike {
  from_tx_id: number;
  to_tx_id: number;
  tx_count?: number;
  gross_spent: number;
  operator_share: number;
  protocol_fee: number;
  reserve_locked?: number;
}

function mapReplaySummary(r: ReplaySummaryLike) {
  return {
    from_tx_id: r.from_tx_id,
    to_tx_id: r.to_tx_id,
    tx_count: r.tx_count ?? 0,
    gross_spent: r.gross_spent,
    operator_share: r.operator_share,
    protocol_fee: r.protocol_fee,
    reserve_locked: r.reserve_locked ?? 0,
  };
}

export function createCliAdapter(config: CliAdapterConfig = {}): FrontendDataAdapter {
  const { dataDir, run: runFn } = config;
  const baseArgs = dataDir ? ['--data-dir', dataDir] : [];
  const run = runFn ?? defaultRun;

  const exec = async (args: string[]): Promise<{ stdout: string; stderr: string; exitCode: number }> => {
    return run([...baseArgs, ...args], dataDir ? { cwd: dataDir } : undefined);
  };

  const adapter: FrontendDataAdapter = {
    readonlyMode: false,
    async listSettlements(filters?: ListSettlementsFilters) {
      const args = ['settlement', 'list'];
      if (filters?.owner) args.push('--owner', filters.owner);
      if (filters?.service_id) args.push('--service-id', filters.service_id);
      if (filters?.status) args.push('--status', filters.status);
      const { stdout, stderr, exitCode } = await exec(args);
      if (exitCode !== 0) throw parseCliStderr(stderr);
      const data = JSON.parse(stdout || '{"settlements":[]}') as { settlements: unknown[] };
      return (data.settlements || []).map((s) => mapListToSettlementView(s as Parameters<typeof mapListToSettlementView>[0]));
    },
    async getSettlement(owner, serviceId, windowId) {
      const { stdout, exitCode } = await exec([
        'settlement', 'show', '--owner', owner, '--service-id', serviceId, '--window-id', windowId,
      ]);
      if (exitCode !== 0) return null;
      try {
        return mapShowToSettlementView(JSON.parse(stdout) as Parameters<typeof mapShowToSettlementView>[0]);
      } catch {
        return null;
      }
    },
    async listClaims() {
      // CLI has no generic claim list; derive from settlements or return []
      return Promise.resolve([]);
    },
    async getDispute(owner, serviceId, windowId) {
      const { stdout, exitCode } = await exec([
        'settlement', 'dispute-show', '--owner', owner, '--service-id', serviceId, '--window-id', windowId,
      ]);
      if (exitCode !== 0) return null;
      try {
        const o = JSON.parse(stdout) as { settlement_key: string; status: string; resolution_audit?: { replay_hash: string; replay_summary: ReplaySummaryLike } };
        const d: DisputeView = {
          settlement_key: o.settlement_key,
          status: o.status,
          resolution_audit: o.resolution_audit
            ? { replay_hash: o.resolution_audit.replay_hash, replay_summary: mapReplaySummary(o.resolution_audit.replay_summary) }
            : null,
        };
        return d;
      } catch {
        return null;
      }
    },
    async getEvidenceBundle(owner, serviceId, windowId) {
      const { stdout, exitCode } = await exec([
        'settlement', 'evidence-show', '--owner', owner, '--service-id', serviceId, '--window-id', windowId,
      ]);
      if (exitCode !== 0) return null;
      try {
        const o = JSON.parse(stdout) as {
          settlement_key: string;
          from_tx_id: number;
          to_tx_id: number;
          evidence_hash: string;
          replay_hash: string;
          replay_summary: ReplaySummaryLike;
        };
        return {
          settlement_key: o.settlement_key,
          from_tx_id: o.from_tx_id,
          to_tx_id: o.to_tx_id,
          evidence_hash: o.evidence_hash,
          replay_hash: o.replay_hash,
          replay_summary: mapReplaySummary(o.replay_summary),
        };
      } catch {
        return null;
      }
    },
    async listPolicies(_filters?: ListPolicyFilters) {
      // CLI policy list prints human format; return [] until backend adds JSON
      return Promise.resolve([]);
    },
    async getPolicy() {
      return Promise.resolve(null);
    },
    async finalizeSettlement(owner, serviceId, windowId) {
      const { stderr, exitCode } = await exec([
        'settlement', 'finalize', '--owner', owner, '--service-id', serviceId, '--window-id', windowId, '--allow-unsigned',
      ]);
      if (exitCode !== 0) return parseCliStderr(stderr);
      return { ok: true };
    },
    async submitClaim(operator, owner, serviceId, windowId, amount) {
      const { stderr, exitCode } = await exec([
        'settlement', 'claim-submit', '--operator', operator, '--owner', owner, '--service-id', serviceId,
        '--window-id', windowId, '--amount', String(amount), '--allow-unsigned',
      ]);
      if (exitCode !== 0) return parseCliStderr(stderr);
      return { ok: true };
    },
    async payClaim(operator, owner, serviceId, windowId) {
      const { stderr, exitCode } = await exec([
        'settlement', 'claim-pay', '--operator', operator, '--owner', owner, '--service-id', serviceId,
        '--window-id', windowId, '--allow-unsigned',
      ]);
      if (exitCode !== 0) return parseCliStderr(stderr);
      return { ok: true };
    },
    async openDispute(owner, serviceId, windowId, reasonCode) {
      const args = ['settlement', 'open-dispute', '--owner', owner, '--service-id', serviceId, '--window-id', windowId, '--allow-unsigned'];
      if (reasonCode) args.push('--reason', reasonCode);
      const { stderr, exitCode } = await exec(args);
      if (exitCode !== 0) return parseCliStderr(stderr);
      return { ok: true };
    },
    async resolveDispute(owner, serviceId, windowId, verdict) {
      const { stderr, exitCode } = await exec([
        'settlement', 'resolve-dispute', '--owner', owner, '--service-id', serviceId, '--window-id', windowId,
        '--verdict', verdict, '--allow-unsigned',
      ]);
      if (exitCode !== 0) return parseCliStderr(stderr);
      return { ok: true };
    },
    async publishPolicy(params) {
      const { stderr, exitCode } = await exec([
        'policy', 'publish', '--scope', params.scope, '--version', String(params.version),
        '--effective-from-tx-id', String(params.effective_from_tx_id),
        '--operator-share-bps', String(params.operator_share_bps),
        '--protocol-fee-bps', String(params.protocol_fee_bps),
        '--dispute-window-secs', String(params.dispute_window_secs),
        '--reserve-fixed', String(params.reserve_fixed ?? 0), '--signer', 'authority', '--allow-unsigned',
      ]);
      if (exitCode !== 0) return parseCliStderr(stderr);
      return { ok: true };
    },
  };
  return adapter;
}
