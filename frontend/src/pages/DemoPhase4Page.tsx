import { useState, useCallback, useEffect, useMemo } from 'react';
import { useDemoAdapter } from '../adapters/demo-context';
import { createDemoProxyAdapter } from '../adapters/demo-proxy-adapter';
import { DEMO_BYOK_ENABLED, DEMO_PROXY_BASE } from '../config/demo-feature';
import { DemoControlPanel } from '../components/DemoControlPanel';
import { DataTable, type Column } from '../components/DataTable';
import { StatusBadge } from '../components/StatusBadge';
import { ErrorBanner } from '../components/ErrorBanner';
import { EvidenceCompareCard } from '../components/EvidenceCompareCard';
import { CompareStatusChip } from '../components/CompareStatusChip';
import { KpiStrip } from '../components/KpiStrip';
import { formatInteger, formatPercent } from '../utils/format';
import type {
  DemoWindowAggregate,
  DemoUiState,
  DemoCompareStatus,
  EvidenceBundleView,
} from '../domain/types';

const defaultControls: DemoUiState['controls'] = {
  start_date: '2026-02-01',
  end_date: '2026-02-22',
  window_granularity: 'day',
  top_n: 0, // 0 = no filter; real snapshot has operator_count ~80–96 per window
  operator_share_bps: 9000,
  protocol_fee_bps: 1000,
  reserve_bps: 0,
  dispute_window_secs: 86400,
};

function getCompareStatus(
  w: DemoWindowAggregate
): DemoCompareStatus {
  if (!w.replay_summary || !w.replay_hash) return 'MISSING';
  const r = w.replay_summary;
  const match =
    r.gross_spent === w.gross_spent &&
    r.operator_share === w.operator_share &&
    r.protocol_fee === w.protocol_fee &&
    r.reserve_locked === w.reserve_locked;
  return match ? 'MATCH' : 'MISMATCH';
}

function windowToBundle(w: DemoWindowAggregate): EvidenceBundleView | null {
  if (!w.replay_summary || !w.replay_hash || !w.evidence_hash) return null;
  const key = `${w.owner}:${w.service_id}:${w.window_id}`;
  return {
    settlement_key: key,
    from_tx_id: w.from_tx_id ?? 0,
    to_tx_id: w.to_tx_id ?? 0,
    evidence_hash: w.evidence_hash,
    replay_hash: w.replay_hash,
    replay_summary: w.replay_summary,
  };
}

export function DemoPhase4Page() {
  const contextAdapter = useDemoAdapter();
  const [state, setState] = useState<DemoUiState>({
    mode: 'snapshot',
    loading: false,
    controls: defaultControls,
  });
  const [byokKey, setByokKey] = useState('');
  const [windows, setWindows] = useState<DemoWindowAggregate[]>([]);
  const [stale, setStale] = useState(false);

  const effectiveAdapter = useMemo(
    () =>
      state.mode === 'snapshot' || !DEMO_BYOK_ENABLED
        ? contextAdapter
        : createDemoProxyAdapter(DEMO_PROXY_BASE, { getApiKey: () => byokKey || null }),
    [contextAdapter, state.mode, byokKey]
  );

  const recompute = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, last_error: undefined }));
    setStale(false);
    try {
      const list = await effectiveAdapter.getDemoWindows({
        start_date: state.controls.start_date,
        end_date: state.controls.end_date,
        owner: state.controls.owner,
        service_id: state.controls.service_id,
        window_granularity: state.controls.window_granularity,
        operator_share_bps: state.controls.operator_share_bps,
        protocol_fee_bps: state.controls.protocol_fee_bps,
        reserve_bps: state.controls.reserve_bps,
        dispute_window_secs: state.controls.dispute_window_secs,
        top_n: state.controls.top_n,
      });
      setWindows(list);
      setState((s) => ({ ...s, loading: false }));
    } catch (e) {
      setState((s) => ({
        ...s,
        loading: false,
        last_error: { error_code: 'DEMO_LOAD_FAILED', message: String(e) },
      }));
      setStale(true);
    }
  }, [effectiveAdapter, state.controls]);

  useEffect(() => {
    recompute();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- run once on mount with default controls
  }, []);

  const selected = state.selected_window_id
    ? windows.find((w) => `${w.owner}:${w.service_id}:${w.window_id}` === state.selected_window_id)
    : null;
  const compareStatus = selected ? getCompareStatus(selected) : null;
  const bundle = selected ? windowToBundle(selected) : null;
  const canResolve = compareStatus === 'MATCH' && selected?.status?.toLowerCase().includes('disputed');
  const statusStats = useMemo(() => {
    let proposed = 0;
    let finalized = 0;
    let disputed = 0;
    for (const w of windows) {
      const s = (w.status ?? '').toLowerCase();
      if (s.includes('proposed')) proposed += 1;
      else if (s.includes('finalized')) finalized += 1;
      else if (s.includes('disputed')) disputed += 1;
    }
    return { proposed, finalized, disputed };
  }, [windows]);
  const compareStats = useMemo(() => {
    let match = 0;
    let mismatch = 0;
    let missing = 0;
    for (const w of windows) {
      const c = getCompareStatus(w);
      if (c === 'MATCH') match += 1;
      else if (c === 'MISMATCH') mismatch += 1;
      else missing += 1;
    }
    return { match, mismatch, missing };
  }, [windows]);

  const columns: Column<DemoWindowAggregate>[] = [
    {
      key: 'window_id',
      label: 'Window',
      width: '9%',
      render: (r) => {
        const id = `${r.owner}:${r.service_id}:${r.window_id}`;
        return (
          <button
            type="button"
            onClick={() => setState((s) => ({ ...s, selected_window_id: id }))}
            className="cell-nowrap"
            style={{
              background: 'none',
              border: 'none',
              color: 'var(--color-accent)',
              cursor: 'pointer',
              padding: 0,
              textDecoration: state.selected_window_id === id ? 'underline' : 'none',
            }}
          >
            {r.window_id}
          </button>
        );
      },
    },
    { key: 'owner', label: 'Owner', width: '11%', render: (r) => <span className="cell-nowrap">{r.owner.length > 10 ? `${r.owner.slice(0, 8)}…` : r.owner}</span> },
    { key: 'service_id', label: 'Service', width: '8%', render: (r) => <span className="cell-service"><span className="service-badge">{r.service_id}</span></span> },
    { key: 'gross_spent', label: 'Gross spent', width: '13%', render: (r) => <span className="cell-num">{formatInteger(r.gross_spent)}</span> },
    { key: 'operator_share', label: 'Operator share', width: '12%', render: (r) => <span className="cell-num">{formatInteger(r.operator_share)}</span> },
    { key: 'protocol_fee', label: 'Protocol fee', width: '11%', render: (r) => <span className="cell-num">{formatInteger(r.protocol_fee)}</span> },
    { key: 'reserve_locked', label: 'Reserve locked', width: '10%', render: (r) => <span className="cell-num">{formatInteger(r.reserve_locked)}</span> },
    { key: 'top_n_share', label: 'Top-N %', width: '6%', render: (r) => <span className="cell-num">{formatPercent(r.top_n_share)}</span> },
    { key: 'operator_count', label: 'Ops', width: '5%', render: (r) => <span className="cell-num">{formatInteger(r.operator_count)}</span> },
    {
      key: 'status',
      label: 'Status',
      width: '8%',
      render: (r) => (r.status ? <StatusBadge kind="settlement" status={r.status} /> : '—'),
    },
    {
      key: 'compare',
      label: 'Compare',
      width: '8%',
      render: (r) => <CompareStatusChip status={getCompareStatus(r)} />,
    },
  ];

  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-2)' }}>DePIN Helium IOT Settlement Control Plane</h1>
      <p style={{ color: 'var(--color-text-muted)', marginBottom: 'var(--space-4)' }}>
        Raw transfer snapshots enter policy projection, become settlement windows, then pass replay evidence gates before dispute resolution.
      </p>
      <div className="demo-pipeline">
        <div className="demo-stage-card">
          <div className="demo-stage-index">1</div>
          <h4>Source Windows</h4>
          <p>Dune snapshot input filtered by date/owner/service.</p>
          <strong>{formatInteger(windows.length)} windows selected</strong>
        </div>
        <div className="demo-stage-card">
          <div className="demo-stage-index">2</div>
          <h4>Policy Projection</h4>
          <p>Deterministic split from active policy controls.</p>
          <strong>
            {state.controls.operator_share_bps} / {state.controls.protocol_fee_bps} / {state.controls.reserve_bps} bps
          </strong>
        </div>
        <div className="demo-stage-card">
          <div className="demo-stage-index">3</div>
          <h4>Settlement State</h4>
          <p>Economic lifecycle by window status.</p>
          <strong>P:{statusStats.proposed} F:{statusStats.finalized} D:{statusStats.disputed}</strong>
        </div>
        <div className="demo-stage-card">
          <div className="demo-stage-index">4</div>
          <h4>Evidence Gate</h4>
          <p>Replay check decides resolve eligibility.</p>
          <strong>M:{compareStats.match} X:{compareStats.mismatch} ?: {compareStats.missing}</strong>
        </div>
      </div>
      <DemoControlPanel
        state={state}
        onControlsChange={(patch) =>
          setState((s) => ({ ...s, controls: { ...s.controls, ...patch } }))
        }
        onRecompute={recompute}
        byokEnabled={DEMO_BYOK_ENABLED}
        onModeChange={(mode) => setState((s) => ({ ...s, mode }))}
        byokKey={byokKey}
        onByokKeyChange={setByokKey}
      />
      {state.last_error && (
        <ErrorBanner
          error={{
            error_code: state.last_error.error_code,
            message: state.last_error.message,
            suggested_action: 'Check date range and snapshot URL, then Recompute.',
          }}
          onDismiss={() => setState((s) => ({ ...s, last_error: undefined }))}
        />
      )}
      {stale && windows.length > 0 && (
        <p style={{ color: 'var(--color-warning)', marginBottom: 'var(--space-2)' }}>
          Showing previous result; last Recompute failed.
        </p>
      )}
      {windows.length > 0 && <KpiStrip windows={windows} />}
      <div className="demo-layout">
        <div>
          <div className="card">
            <h3>3. Settlement windows</h3>
            {state.loading && windows.length === 0 ? (
              <p style={{ color: 'var(--color-text-muted)' }}>Loading…</p>
            ) : (
              <DataTable
                columns={columns}
                data={windows}
                keyFn={(r) => `${r.owner}:${r.service_id}:${r.window_id}`}
                emptyMessage="No windows in range. Adjust dates and Recompute."
                tableClassName="data-table--fixed data-table--wide"
              />
            )}
          </div>
        </div>
        <div>
          <div className="card">
            <h3>4. Integrity & Evidence</h3>
            {!selected ? (
              <p style={{ color: 'var(--color-text-muted)' }}>Select a window.</p>
            ) : (
              <>
                <p>
                  <strong>{selected.owner}</strong> / {selected.service_id} / {selected.window_id}
                </p>
                <p style={{ color: 'var(--color-text-muted)', marginTop: 'var(--space-1)' }}>
                  Policy snapshot: operator {state.controls.operator_share_bps} bps, protocol {state.controls.protocol_fee_bps} bps, reserve {state.controls.reserve_bps} bps
                </p>
                <p style={{ marginTop: 'var(--space-2)' }}>
                  Compare: <CompareStatusChip status={compareStatus!} />
                </p>
                {bundle && (
                  <EvidenceCompareCard
                    bundle={bundle}
                    recorded={{
                      gross_spent: selected.gross_spent,
                      operator_share: selected.operator_share,
                      protocol_fee: selected.protocol_fee,
                      reserve_locked: selected.reserve_locked,
                    }}
                    mismatch={compareStatus === 'MISMATCH'}
                  />
                )}
                {compareStatus === 'MISSING' && (
                  <p style={{ color: 'var(--color-danger)' }}>
                    Evidence or replay missing. Resolve is blocked. (EVIDENCE_NOT_FOUND)
                  </p>
                )}
                <div style={{ marginTop: 'var(--space-4)' }}>
                  <button
                    type="button"
                    className="primary"
                    disabled={!canResolve}
                    title={!canResolve ? 'Resolve only when compare = MATCH and status = Disputed' : 'Resolve dispute'}
                  >
                    Resolve Dispute
                  </button>
                  {!canResolve && compareStatus === 'MISMATCH' && selected.status?.toLowerCase() !== 'disputed' && (
                    <p style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)', marginTop: 'var(--space-1)' }}>
                      Window is not in Disputed status.
                    </p>
                  )}
                </div>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
