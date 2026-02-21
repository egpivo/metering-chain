import { useEffect, useMemo, useState } from 'react';
import { ErrorBanner } from '../components/ErrorBanner';

interface SnapshotWindow {
  owner: string;
  service_id: string;
  window_id: string;
  gross_spent: number;
  status?: string;
}

interface SnapshotPayload {
  version?: number;
  generated_at?: string;
  windows: SnapshotWindow[];
  usage_rows?: unknown[];
}

export function AuditDataPage() {
  const [data, setData] = useState<SnapshotPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<{ error_code: string; message: string; suggested_action: string } | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    fetch('/demo_data/phase4_snapshot.json')
      .then(async (res) => {
        if (!res.ok) throw new Error(`snapshot load failed: ${res.status}`);
        return res.json() as Promise<SnapshotPayload>;
      })
      .then((payload) => {
        if (!cancelled) setData(payload);
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          setError({
            error_code: 'SNAPSHOT_LOAD_FAILED',
            message: String(e),
            suggested_action: 'Run frontend/server/refresh_demo_snapshot.sh and refresh.',
          });
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const summary = useMemo(() => {
    const windows = data?.windows ?? [];
    const owners = new Set(windows.map((w) => w.owner));
    const services = new Set(windows.map((w) => w.service_id));
    const gross = windows.reduce((s, w) => s + (w.gross_spent || 0), 0);
    return {
      windows: windows.length,
      owners: owners.size,
      services: services.size,
      gross,
    };
  }, [data]);

  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-4)' }}>Data Source</h1>
      <div className="card">
        <h3>Source</h3>
        <p style={{ margin: 0, color: 'var(--color-text-muted)' }}>
          This app reads DePIN Helium IOT snapshot data from <code className="mono">frontend/public/demo_data/phase4_snapshot.json</code>, built from Dune transfer exports.
        </p>
      </div>

      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
      {loading && <p style={{ color: 'var(--color-text-muted)' }}>Loading snapshot metadata…</p>}

      {!loading && data && (
        <>
          <div className="card">
            <h3>Snapshot Info</h3>
            <p>Version: {data.version ?? '—'}</p>
            <p>Generated at: {data.generated_at ?? '—'}</p>
            <p>Windows: {summary.windows}</p>
            <p>Owners: {summary.owners}</p>
            <p>Services: {summary.services}</p>
            <p>Total gross spent: <span className="mono">{summary.gross}</span></p>
          </div>
          <div className="card">
            <h3>Refresh</h3>
            <p style={{ margin: 0, color: 'var(--color-text-muted)' }}>
              Refresh real data with <code className="mono">frontend/server/refresh_demo_snapshot.sh</code>.
            </p>
          </div>
        </>
      )}
    </div>
  );
}
