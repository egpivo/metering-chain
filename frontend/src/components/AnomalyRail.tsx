import { useState } from 'react';
import { Link } from 'react-router-dom';
import type { MeteringAnomalyItem } from '../domain/types';

interface AnomalyRailProps {
  count: number;
  items?: MeteringAnomalyItem[];
}

const KIND_STYLE: Record<string, { bg: string; label: string }> = {
  disputed: { bg: 'var(--color-disputed, #c53030)', label: 'Disputed' },
  replay_gap: { bg: 'var(--color-warning, #d69e2e)', label: 'No replay' },
  missing: { bg: 'var(--color-text-muted)', label: 'Missing' },
  spike: { bg: 'var(--color-accent)', label: 'Spike' },
};

function ItemRow({ a, canLink }: { a: MeteringAnomalyItem; canLink: boolean }) {
  const dotColor = (KIND_STYLE[a.kind] ?? {}).bg ?? 'var(--color-border)';
  const content = (
    <>
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: '50%',
          backgroundColor: dotColor,
          flexShrink: 0,
        }}
      />
      {a.label}
    </>
  );
  if (canLink && a.owner && a.service_id && a.window_id) {
    return (
      <Link
        to={`/settlements/${a.owner}/${a.service_id}/${a.window_id}`}
        style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}
      >
        {content}
      </Link>
    );
  }
  return <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>{content}</span>;
}

export function AnomalyRail({ count, items = [] }: AnomalyRailProps) {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const byKind = items.reduce<Record<string, number>>((acc, a) => {
    acc[a.kind] = (acc[a.kind] ?? 0) + 1;
    return acc;
  }, {});

  const byDay = items.reduce<Record<string, { disputed: number; replay_gap: number }>>((acc, a) => {
    const day = a.window_id ?? a.id.split('-')[0] ?? '';
    if (!day) return acc;
    if (!acc[day]) acc[day] = { disputed: 0, replay_gap: 0 };
    if (a.kind === 'disputed') acc[day].disputed += 1;
    else if (a.kind === 'replay_gap') acc[day].replay_gap += 1;
    return acc;
  }, {});
  const sortedDays = Object.keys(byDay).sort();
  const canLink = (a: MeteringAnomalyItem) => Boolean(a.owner && a.service_id && a.window_id);

  return (
    <section className="anomaly-rail card" style={{ marginBottom: 'var(--space-4)' }} aria-label="Anomaly rail">
      <h3 style={{ marginTop: 0, fontSize: 'var(--text-base)' }}>Anomaly rail</h3>
      {count === 0 ? (
        <p style={{ margin: 0, color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>
          No anomalies in range.
        </p>
      ) : (
        <>
          {/* Kind breakdown */}
          <div className="anomaly-rail__summary" style={{ display: 'flex', flexWrap: 'wrap', gap: 'var(--space-2)', marginBottom: 'var(--space-3)' }}>
            {Object.entries(byKind).map(([kind, n]) => {
              const style = KIND_STYLE[kind] ?? { bg: 'var(--color-border)', label: kind };
              return (
                <span
                  key={kind}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 6,
                    fontSize: 'var(--text-sm)',
                    padding: '4px 10px',
                    borderRadius: 'var(--radius)',
                    background: style.bg,
                    color: '#fff',
                    fontWeight: 600,
                  }}
                >
                  <span style={{ opacity: 0.9 }}>{style.label}</span>
                  <span>{n}</span>
                </span>
              );
            })}
          </div>

          {/* Day strip: which days have anomalies */}
          {sortedDays.length > 0 && (
            <div className="anomaly-rail__day-strip" style={{ marginBottom: 'var(--space-3)' }}>
              <p style={{ margin: '0 0 var(--space-1)', fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                By day (red = disputed, orange = no replay)
              </p>
              <div
                style={{
                  display: 'flex',
                  flexWrap: 'wrap',
                  gap: 3,
                  alignItems: 'center',
                }}
                role="img"
                aria-label={`${sortedDays.length} days with anomalies`}
              >
                {sortedDays.map((day) => {
                  const d = byDay[day];
                  const hasDisputed = d.disputed > 0;
                  const color = hasDisputed
                    ? 'var(--color-disputed, #c53030)'
                    : 'var(--color-warning, #d69e2e)';
                  const title = hasDisputed
                    ? `${day}: ${d.disputed} disputed, ${d.replay_gap} no replay`
                    : `${day}: ${d.replay_gap} no replay`;
                  return (
                    <span
                      key={day}
                      title={title}
                      style={{
                        display: 'inline-block',
                        width: 20,
                        height: 20,
                        minWidth: 20,
                        minHeight: 20,
                        borderRadius: 3,
                        backgroundColor: color,
                        opacity: 0.9,
                      }}
                    />
                  );
                })}
              </div>
            </div>
          )}

          {/* Collapsible detailed list */}
          <div>
            <button
              type="button"
              onClick={() => setDetailsOpen((o) => !o)}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 6,
                marginBottom: detailsOpen ? 'var(--space-2)' : 0,
                padding: '6px 12px',
                fontSize: 'var(--text-sm)',
                color: 'var(--color-accent)',
                background: 'transparent',
                border: '1px solid var(--color-border)',
                borderRadius: 'var(--radius)',
                cursor: 'pointer',
              }}
              aria-expanded={detailsOpen}
            >
              <strong>{count}</strong> item{count !== 1 ? 's' : ''} — click to open settlement
              <span style={{ marginLeft: 4 }}>{detailsOpen ? '▲ Hide list' : '▼ Show full list'}</span>
            </button>
            {detailsOpen && (
              <ul
                style={{
                  margin: 0,
                  paddingLeft: 'var(--space-5)',
                  fontSize: 'var(--text-sm)',
                  maxHeight: 320,
                  overflowY: 'auto',
                  border: '1px solid var(--color-border)',
                  borderRadius: 'var(--radius)',
                  paddingTop: 'var(--space-2)',
                  paddingBottom: 'var(--space-2)',
                }}
              >
                {items.map((a) => (
                  <li key={a.id} style={{ marginBottom: 'var(--space-1)' }}>
                    <ItemRow a={a} canLink={canLink(a)} />
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      )}
    </section>
  );
}
