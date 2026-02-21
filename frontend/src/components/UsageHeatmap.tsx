import type { MeteringSeriesPoint } from '../domain/types';

interface UsageHeatmapProps {
  series: MeteringSeriesPoint[];
  /** Day or week */
  granularity: 'day' | 'week';
}

export function UsageHeatmap({ series, granularity }: UsageHeatmapProps) {
  if (series.length === 0) return null;
  const maxCost = Math.max(...series.map((p) => p.cost), 1);
  const cellSize = 12;
  const gap = 2;

  return (
    <div className="usage-heatmap" style={{ marginTop: 'var(--space-2)' }}>
      <div
        style={{
          display: 'flex',
          flexWrap: 'wrap',
          gap,
          alignItems: 'flex-end',
        }}
        role="img"
        aria-label={`Usage by ${granularity}`}
      >
        {series.map((p) => {
          const intensity = maxCost > 0 ? p.cost / maxCost : 0;
          const opacity = 0.2 + 0.8 * intensity;
          return (
            <div
              key={p.ts}
              title={`${p.ts.slice(0, 10)}: ${p.cost} (${p.window_count ?? 0} windows)`}
              style={{
                width: cellSize,
                height: cellSize,
                minWidth: cellSize,
                minHeight: cellSize,
                backgroundColor: 'var(--color-accent)',
                opacity,
                borderRadius: 2,
              }}
            />
          );
        })}
      </div>
      <p style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)', marginTop: 'var(--space-1)', marginBottom: 0 }}>
        {series.length} {granularity}s · darker = higher spend
      </p>
    </div>
  );
}
