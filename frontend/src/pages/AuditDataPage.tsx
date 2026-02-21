import { Link } from 'react-router-dom';

/**
 * Data Source page: credit and refresh. Snapshot details live on Overview to avoid duplication.
 */
export function AuditDataPage() {
  return (
    <div>
      <h1 style={{ marginBottom: 'var(--space-4)' }}>Data Source</h1>
      <div className="card">
        <h3 style={{ marginTop: 0 }}>Credit</h3>
        <p style={{ margin: 0 }}>
          Demo data is sourced from <a href="https://dune.com" target="_blank" rel="noopener noreferrer">Dune</a> (Helium IOT transfer queries). Data by Dune.
        </p>
      </div>
      <div className="card" style={{ marginTop: 'var(--space-4)' }}>
        <h3 style={{ marginTop: 0 }}>Snapshot &amp; refresh</h3>
        <p style={{ marginBottom: 'var(--space-3)' }}>
          Snapshot version, generated time, window/owner counts, and total gross are shown on the <Link to="/overview#data-source">Overview page (Data Source block)</Link>.
        </p>
        <p style={{ margin: 0, color: 'var(--color-text-muted)' }}>
          To refresh: run <code className="mono">frontend/server/refresh_demo_snapshot.sh</code> (requires <code className="mono">DUNE_API_KEY</code> in <code className="mono">.env</code>).
        </p>
      </div>
    </div>
  );
}
