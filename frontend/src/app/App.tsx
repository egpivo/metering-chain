import { Routes, Route } from 'react-router-dom';
import { Layout } from './layout';
import { SettlementsPage } from '../pages/SettlementsPage';
import { SettlementDetailPage } from '../pages/SettlementDetailPage';
import { ClaimsPage } from '../pages/ClaimsPage';
import { DisputesPage } from '../pages/DisputesPage';
import { PolicyPage } from '../pages/PolicyPage';
import { OverviewPage } from '../pages/OverviewPage';
import { MeteringPage } from '../pages/MeteringPage';
import { DemoPhase4Page } from '../pages/DemoPhase4Page';
import { AuditDataPage } from '../pages/AuditDataPage';
import { DemoAdapterProvider } from '../adapters/demo-context';
import { DemoSnapshotAdapter } from '../adapters/demo-snapshot-adapter';

export default function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<OverviewPage />} />
        <Route path="/overview" element={<OverviewPage />} />
        <Route path="/metering" element={<MeteringPage />} />
        <Route path="/settlements" element={<SettlementsPage />} />
        <Route path="/settlements/:owner/:serviceId/:windowId" element={<SettlementDetailPage />} />
        <Route path="/claims" element={<ClaimsPage />} />
        <Route path="/disputes" element={<DisputesPage />} />
        <Route path="/policy" element={<PolicyPage />} />
        <Route
          path="/audit/explorer"
          element={
            <DemoAdapterProvider adapter={DemoSnapshotAdapter}>
              <DemoPhase4Page />
            </DemoAdapterProvider>
          }
        />
        <Route
          path="/demo/phase4"
          element={
            <DemoAdapterProvider adapter={DemoSnapshotAdapter}>
              <DemoPhase4Page />
            </DemoAdapterProvider>
          }
        />
        <Route path="/audit/data" element={<AuditDataPage />} />
      </Routes>
    </Layout>
  );
}
