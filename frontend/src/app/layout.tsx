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
          <ul>
            <li><NavLink to="/overview" end className={({ isActive }) => isActive ? 'active' : ''}>Overview</NavLink></li>
            <li><NavLink to="/metering" className={({ isActive }) => isActive ? 'active' : ''}>Metering</NavLink></li>
            <li><NavLink to="/settlements" className={({ isActive }) => isActive ? 'active' : ''}>Settlements</NavLink></li>
            <li><NavLink to="/disputes" className={({ isActive }) => isActive ? 'active' : ''}>Disputes</NavLink></li>
            <li><NavLink to="/audit/explorer" className={({ isActive }) => isActive ? 'active' : ''}>Audit</NavLink></li>
            <li><NavLink to="/policy" className={({ isActive }) => isActive ? 'active' : ''}>Policy</NavLink></li>
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
