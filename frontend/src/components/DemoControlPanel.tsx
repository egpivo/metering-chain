import type { DemoUiState } from '../domain/types';

interface DemoControlPanelProps {
  state: DemoUiState;
  onControlsChange: (patch: Partial<DemoUiState['controls']>) => void;
  onRecompute: () => void;
  onModeChange?: (mode: 'snapshot' | 'byok') => void;
  byokEnabled?: boolean;
  byokKey?: string;
  onByokKeyChange?: (value: string) => void;
}

export function DemoControlPanel({
  state,
  onControlsChange,
  onRecompute,
  onModeChange,
  byokEnabled,
  byokKey = '',
  onByokKeyChange,
}: DemoControlPanelProps) {
  const c = state.controls;
  return (
    <div className="card">
      <h3>Demo controls</h3>
      {byokEnabled && onModeChange && (
        <div style={{ marginBottom: 'var(--space-4)' }}>
          <span style={{ marginRight: 'var(--space-2)', color: 'var(--color-text-muted)' }}>Dataset mode:</span>
          <button
            type="button"
            onClick={() => onModeChange('snapshot')}
            className={state.mode === 'snapshot' ? 'primary' : ''}
            style={{ marginRight: 'var(--space-2)' }}
          >
            Snapshot
          </button>
          <button
            type="button"
            onClick={() => onModeChange('byok')}
            className={state.mode === 'byok' ? 'primary' : ''}
          >
            Use my Dune key
          </button>
          {state.mode === 'byok' && onByokKeyChange && (
            <div style={{ marginTop: 'var(--space-3)' }}>
              <label>
                API key (session only, sent to proxy only)
                <input
                  type="password"
                  autoComplete="off"
                  placeholder="Dune API key"
                  value={byokKey}
                  onChange={(e) => onByokKeyChange(e.target.value)}
                  style={{ maxWidth: 280, marginLeft: 'var(--space-2)' }}
                />
              </label>
              <p style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', marginTop: 'var(--space-1)' }}>
                Key is not stored; it is sent only to the demo proxy. Use at your own risk.
              </p>
            </div>
          )}
        </div>
      )}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-4)', alignItems: 'flex-end' }}>
        <label>
          Start date
          <input
            type="date"
            value={c.start_date}
            onChange={(e) => onControlsChange({ start_date: e.target.value })}
          />
        </label>
        <label>
          End date
          <input
            type="date"
            value={c.end_date}
            onChange={(e) => onControlsChange({ end_date: e.target.value })}
          />
        </label>
        <label>
          Granularity
          <select
            value={c.window_granularity}
            onChange={(e) => onControlsChange({ window_granularity: e.target.value as 'day' | 'week' })}
          >
            <option value="day">Day</option>
            <option value="week">Week</option>
          </select>
        </label>
        <label>
          Owner
          <input
            placeholder="Filter owner"
            value={c.owner ?? ''}
            onChange={(e) => onControlsChange({ owner: e.target.value || undefined })}
          />
        </label>
        <label>
          Service
          <input
            placeholder="Filter service"
            value={c.service_id ?? ''}
            onChange={(e) => onControlsChange({ service_id: e.target.value || undefined })}
          />
        </label>
        <label>
          Operator share (bps)
          <input
            type="number"
            min={0}
            max={10000}
            value={c.operator_share_bps}
            onChange={(e) => onControlsChange({ operator_share_bps: Number(e.target.value) || 0 })}
          />
        </label>
        <label>
          Protocol fee (bps)
          <input
            type="number"
            min={0}
            max={10000}
            value={c.protocol_fee_bps}
            onChange={(e) => onControlsChange({ protocol_fee_bps: Number(e.target.value) || 0 })}
          />
        </label>
        <label>
          Reserve (bps)
          <input
            type="number"
            min={0}
            max={10000}
            value={c.reserve_bps}
            onChange={(e) => onControlsChange({ reserve_bps: Number(e.target.value) || 0 })}
          />
        </label>
        <label title="Not used in backend or recompute yet; for display only.">
          Dispute window (secs) <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-xs)' }}>(preview only)</span>
          <input
            type="number"
            min={0}
            value={c.dispute_window_secs}
            onChange={(e) => onControlsChange({ dispute_window_secs: Number(e.target.value) || 0 })}
          />
        </label>
        <label title="Filter: only show windows with at most this many operators. 0 = no filter. Not Top-N concentration share.">
          Max operators per window
          <input
            type="number"
            min={0}
            placeholder="0 = all"
            value={c.top_n}
            onChange={(e) => onControlsChange({ top_n: Number(e.target.value) || 0 })}
          />
        </label>
        <button type="button" className="primary" onClick={onRecompute} disabled={state.loading}>
          {state.loading ? 'Loading…' : 'Recompute'}
        </button>
      </div>
    </div>
  );
}
