import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { MeteringSnapshotAdapter } from '../adapters/metering-snapshot-adapter';
import { formatInteger } from '../utils/format';
import type {
  MeteringCounters,
  MeteringSeriesPoint,
  MeteringTopOperator,
  MeteringWindowPreview,
} from '../domain/types';

const DEFAULT_START = '2026-02-01';
const DEFAULT_END = '2026-02-22';

export function MeteringPage() {
  const [startDate, setStartDate] = useState(DEFAULT_START);
  const [endDate, setEndDate] = useState(DEFAULT_END);
  const [granularity, setGranularity] = useState<'day' | 'week'>('day');
  const [counters, setCounters] = useState<MeteringCounters | null>(null);
  const [series, setSeries] = useState<MeteringSeriesPoint[]>([]);
  const [topOperators, setTopOperators] = useState<MeteringTopOperator[]>([]);
  const [windowPreview, setWindowPreview] = useState<{ count: number; windows: MeteringWindowPreview[] } | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    const adapter = MeteringSnapshotAdapter;
    const params = { start_date: startDate, end_date: endDate };

    Promise.all([
      adapter.getMeteringCounters(params),
      adapter.getMeteringSeries({ ...params, granularity }),
      adapter.getMeteringTopOperators({ ...params, limit: 10 }),
      adapter.getWindowPreview(params),
    ])
      .then(([c, s, t, w]) => {
        if (!cancelled) {
          setCounters(c);
          setSeries(s);
          setTopOperators(t);
          setWindowPreview(w);
        }
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => { cancelled = true; };
  }, [startDate, endDate, granularity]);

  const maxCost = series.length ? Math.max(...series.map((p) => p.cost), 1) : 1;

  return (
    <div className="metering-page">
      <h1 style={{ marginBottom: 'var(--space-2)' }}>Metering</h1>
      <p style={{ color: 'var(--color-text-muted)', marginBottom: 'var(--space-4)' }}>
        Usage and windows that feed settlement. Data from Dune snapshot (Helium IOT).
      </p>

      <div className="card" style={{ marginBottom: 'var(--space-4)' }}>
        <h3 style={{ marginTop: 0 }}>Date range</h3>
        <div style={{ display: 'flex', gap: 'var(--space-4)', flexWrap: 'wrap', alignItems: 'center' }}>
          <label>
            Start <input type="date" value={startDate} onChange={(e) => setStartDate(e.target.value)} />
          </label>
          <label>
            End <input type="date" value={endDate} onChange={(e) => setEndDate(e.target.value)} />
          </label>
          <label>
            Granularity{' '}
            <select value={granularity} onChange={(e) => setGranularity(e.target.value as 'day' | 'week')}>
              <option value="day">Day</option>
              <option value="week">Week</option>
            </select>
          </label>
        </div>
      </div>

      {error && (
        <div className="card" style={{ marginBottom: 'var(--space-4)', borderColor: 'var(--color-error)', background: 'var(--color-error-bg, rgba(207,34,46,0.08))' }}>
          <strong>Error:</strong> {error}
        </div>
      )}

      {loading && (
        <p style={{ color: 'var(--color-text-muted)' }}>Loading…</p>
      )}

      {!loading && counters && (
        <>
          {/* Meter counters */}
          <section className="card" style={{ marginBottom: 'var(--space-4)' }} aria-label="Meter counters">
            <h3 style={{ marginTop: 0 }}>Meter counters</h3>
            <div className="metering-counters" style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-6)' }}>
              <div>
                <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Total spent (range) </span>
                <strong className="mono">{formatInteger(counters.total_cost ?? 0)}</strong>
              </div>
              <div>
                <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Active owners </span>
                <strong>{counters.active_operators}</strong>
              </div>
              <div>
                <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Windows in range </span>
                <strong>{counters.windows_in_range}</strong>
              </div>
              <div>
                <span style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>Anomalies </span>
                <strong>{counters.anomalies}</strong>
              </div>
            </div>
          </section>

          {/* Usage timeline */}
          <section className="card" style={{ marginBottom: 'var(--space-4)' }} aria-label="Usage timeline">
            <h3 style={{ marginTop: 0 }}>Usage timeline</h3>
            {series.length === 0 ? (
              <p style={{ color: 'var(--color-text-muted)' }}>No data in this range.</p>
            ) : (
              <div className="metering-timeline" style={{ display: 'flex', alignItems: 'flex-end', gap: 4, minHeight: 120, flexWrap: 'wrap' }}>
                {series.map((p) => (
                  <div
                    key={p.ts}
                    title={`${p.ts.slice(0, 10)}: ${formatInteger(p.cost)} (${p.window_count ?? 0} windows)`}
                    style={{
                      width: granularity === 'day' ? 28 : 56,
                      height: Math.max(8, (p.cost / maxCost) * 100),
                      background: 'var(--color-accent)',
                      borderRadius: 'var(--radius)',
                      minWidth: 0,
                    }}
                  />
                ))}
              </div>
            )}
            <div style={{ display: 'flex', gap: 'var(--space-2)', marginTop: 'var(--space-2)', fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)' }}>
              {series.slice(0, 5).map((p) => (
                <span key={p.ts}>{p.ts.slice(0, 10)}</span>
              ))}
              {series.length > 5 && <span>…</span>}
            </div>
          </section>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: 'var(--space-4)' }}>
            {/* Top operators */}
            <section className="card" aria-label="Top operators">
              <h3 style={{ marginTop: 0 }}>Top operators</h3>
              {topOperators.length === 0 ? (
                <p style={{ color: 'var(--color-text-muted)' }}>None in range.</p>
              ) : (
                <table className="data-table" style={{ width: '100%' }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: 'left' }}>#</th>
                      <th style={{ textAlign: 'left' }}>Owner</th>
                      <th style={{ textAlign: 'left' }}>Service</th>
                      <th style={{ textAlign: 'right' }}>Cost</th>
                    </tr>
                  </thead>
                  <tbody>
                    {topOperators.map((o, i) => (
                      <tr key={o.owner}>
                        <td>{i + 1}</td>
                        <td className="cell-nowrap" style={{ maxWidth: 140, overflow: 'hidden', textOverflow: 'ellipsis' }} title={o.owner}>
                          {o.owner.length > 12 ? `${o.owner.slice(0, 10)}…` : o.owner}
                        </td>
                        <td><span className="service-badge">{o.service_id}</span></td>
                        <td style={{ textAlign: 'right' }} className="cell-num">{formatInteger(o.cost)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>

            {/* Window preview */}
            <section className="card" aria-label="Window preview">
              <h3 style={{ marginTop: 0 }}>Window preview</h3>
              <p style={{ color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)', marginBottom: 'var(--space-3)' }}>
                With current filters you get <strong>{windowPreview?.count ?? 0}</strong> windows.
              </p>
              {windowPreview && windowPreview.windows.length > 0 ? (
                <>
                  <table className="data-table" style={{ width: '100%', marginBottom: 'var(--space-3)' }}>
                    <thead>
                      <tr>
                        <th style={{ textAlign: 'left' }}>Window</th>
                        <th style={{ textAlign: 'right' }}>Operators</th>
                        <th style={{ textAlign: 'right' }}>Gross</th>
                      </tr>
                    </thead>
                    <tbody>
                      {windowPreview.windows.map((w) => (
                        <tr key={`${w.owner ?? ''}-${w.window_id}`}>
                          <td className="cell-nowrap">{w.window_id}</td>
                          <td style={{ textAlign: 'right' }}>{w.operator_count}</td>
                          <td style={{ textAlign: 'right' }} className="cell-num">{formatInteger(w.gross_spent)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  <Link to="/settlements" className="button primary" style={{ display: 'inline-block' }}>
                    View all in Settlements →
                  </Link>
                </>
              ) : (
                <Link to="/settlements" className="button primary" style={{ display: 'inline-block' }}>
                  Open Settlements
                </Link>
              )}
            </section>
          </div>
        </>
      )}
    </div>
  );
}
