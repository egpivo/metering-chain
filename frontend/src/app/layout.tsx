import { Outlet } from 'react-router-dom';
import { NavLink } from 'react-router-dom';

export function Layout({ children }: { children?: React.ReactNode }) {
  return (
    <div className="app-shell">
      <header className="app-banner">
        <div className="app-brand-title">Metering Chain</div>
        <div className="app-brand-tags">
          <span className="app-tag">Helium IOT</span>
          <span className="app-tag">Dune snapshot</span>
        </div>
      </header>
      <div className="app-body">
        <nav className="app-nav">
          <h2>Operations</h2>
        <ul>
          <li><NavLink to="/settlements" end className={({ isActive }) => isActive ? 'active' : ''}>Settlements</NavLink></li>
          <li><NavLink to="/claims" className={({ isActive }) => isActive ? 'active' : ''}>Claims</NavLink></li>
          <li><NavLink to="/disputes" className={({ isActive }) => isActive ? 'active' : ''}>Disputes</NavLink></li>
        </ul>
        <h2>Governance</h2>
        <ul>
          <li><NavLink to="/policy" className={({ isActive }) => isActive ? 'active' : ''}>Policy</NavLink></li>
        </ul>
        <h2>Audit</h2>
        <ul>
          <li><NavLink to="/audit/explorer" className={({ isActive }) => isActive ? 'active' : ''}>Explorer</NavLink></li>
          <li><NavLink to="/audit/data" className={({ isActive }) => isActive ? 'active' : ''}>Data Source</NavLink></li>
        </ul>
        </nav>
        <main className="app-main">
          {children ?? <Outlet />}
        </main>
      </div>
    </div>
  );
}
