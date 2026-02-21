import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { formatInteger } from '../utils/format';

interface SnapshotPayload {
  version?: number;
  generated_at?: string;
  windows?: { owner: string; service_id: string; gross_spent?: number }[];
}

export function OverviewPage() {
  const [snapshot, setSnapshot] = useState<SnapshotPayload | null>(null);
  const [snapshotError, setSnapshotError] = useState<string | null>(null);

  useEffect(() => {
    if (window.location.hash === '#data-source') {
      document.getElementById('data-source')?.scrollIntoView({ behavior: 'smooth' });
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    fetch('/demo_data/phase4_snapshot.json')
      .then((res) => {
        if (!res.ok) throw new Error(`Snapshot load failed: ${res.status}`);
        return res.json() as Promise<SnapshotPayload>;
      })
      .then((data) => { if (!cancelled) setSnapshot(data); })
      .catch((e) => { if (!cancelled) setSnapshotError(e instanceof Error ? e.message : String(e)); });
    return () => { cancelled = true; };
  }, []);

  const snapshotSummary = useMemo(() => {
    const windows = snapshot?.windows ?? [];
    const owners = new Set(windows.map((w) => w.owner));
    const services = new Set(windows.map((w) => w.service_id));
    const gross = windows.reduce((s, w) => s + (w.gross_spent ?? 0), 0);
    return { windows: windows.length, owners: owners.size, services: services.size, gross };
  }, [snapshot]);

  return (
    <div className="overview-page">
      <h1 style={{ marginBottom: 'var(--space-2)' }}>Start Here</h1>

      {/* What this is */}
      <section className="card overview-block" style={{ marginBottom: 'var(--space-4)' }}>
        <h2 style={{ marginTop: 0, fontSize: 'var(--text-lg)' }}>What this is</h2>
        <p style={{ fontSize: 'var(--text-base)', margin: 0 }}>
          <strong>Metering Chain</strong> turns usage into verifiable settlement and finality — so you can see what was metered, how it was split, and whether evidence matches before resolving disputes.
        </p>
      </section>

      {/* How to read this site */}
      <section className="card overview-block" style={{ marginBottom: 'var(--space-4)' }}>
        <h2 style={{ marginTop: 0, fontSize: 'var(--text-lg)' }}>How to read this site</h2>
        <p style={{ color: 'var(--color-text-muted)', marginBottom: 'var(--space-4)' }}>
          Follow the flow in order: metering → settlement → dispute/finality.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
          <div>
            <p style={{ margin: '0 0 var(--space-2)' }}>
              <strong>Metering</strong> — What we meter: usage timeline, top operators, and the windows that feed settlement.
            </p>
            <Link to="/metering" className="button primary">Go to Metering</Link>
          </div>
          <div>
            <p style={{ margin: '0 0 var(--space-2)' }}>
              <strong>Settlement</strong> — Economic split and lifecycle: proposed → finalized → claimed or disputed.
            </p>
            <Link to="/settlements" className="button primary">Go to Settlements</Link>
          </div>
          <div>
            <p style={{ margin: '0 0 var(--space-2)' }}>
              <strong>Dispute / Finality</strong> — Evidence vs replay; resolve is gated by match. Disputes and policy live here.
            </p>
            <Link to="/disputes" className="button primary">Go to Disputes</Link>
          </div>
        </div>
      </section>

      {/* Data Source — single place for snapshot info + Dune credit */}
      <section className="card overview-block" id="data-source" style={{ marginBottom: 'var(--space-4)' }}>
        <h2 style={{ marginTop: 0, fontSize: 'var(--text-lg)' }}>Data Source</h2>
        <p style={{ marginBottom: 'var(--space-3)' }}>
          This demo uses transfer data from <a href="https://dune.com" target="_blank" rel="noopener noreferrer">Dune</a> (Helium IOT). The site loads a snapshot file (<code className="mono">phase4_snapshot.json</code>); pipeline: Dune query → CSV → snapshot JSON.
        </p>
        <p style={{ marginBottom: 'var(--space-3)', color: 'var(--color-text-muted)', fontSize: 'var(--text-sm)' }}>
          <strong>Credit:</strong> Data by <a href="https://dune.com" target="_blank" rel="noopener noreferrer">Dune</a>.
        </p>
        {snapshotError && (
          <p style={{ color: 'var(--color-error)', marginBottom: 'var(--space-3)' }}>Snapshot: {snapshotError}</p>
        )}
        {snapshot && (
          <div className="snapshot-info" style={{ marginBottom: 'var(--space-3)' }}>
            <h3 style={{ marginTop: 0, fontSize: 'var(--text-base)' }}>Snapshot info</h3>
            <p style={{ margin: 'var(--space-1) 0' }}>Version: {snapshot.version ?? '—'}</p>
            <p style={{ margin: 'var(--space-1) 0' }}>Generated at: {snapshot.generated_at ?? '—'}</p>
            <p style={{ margin: 'var(--space-1) 0' }}>Windows: {snapshotSummary.windows}</p>
            <p style={{ margin: 'var(--space-1) 0' }}>Owners: {snapshotSummary.owners}</p>
            <p style={{ margin: 'var(--space-1) 0' }}>Services: {snapshotSummary.services}</p>
            <p style={{ margin: 'var(--space-1) 0' }}>Total gross spent: <span className="mono">{formatInteger(snapshotSummary.gross)}</span></p>
          </div>
        )}
        <p style={{ color: 'var(--color-text-muted)', marginBottom: 'var(--space-2)', marginTop: 0 }}>
          To refresh the snapshot, run <code className="mono">frontend/server/refresh_demo_snapshot.sh</code> (uses <code className="mono">DUNE_API_KEY</code>).
        </p>
        <p style={{ color: 'var(--color-text-muted)', margin: 0 }}>
          This is snapshot only; no live chain or real-time API in demo mode.
        </p>
      </section>

      {/* Demo vs Production */}
      <section className="card overview-block" style={{ marginBottom: 'var(--space-4)' }}>
        <h2 style={{ marginTop: 0, fontSize: 'var(--text-lg)' }}>Demo vs Production</h2>
        <p style={{ margin: 0, color: 'var(--color-text-muted)' }}>
          This site is read-only (snapshot). To propose, finalize, claim, pay, or resolve disputes you need a live backend (CLI or API).
        </p>
      </section>
    </div>
  );
}
