import { createContext, useContext, type ReactNode } from 'react';
import type { DemoAnalyticsAdapter } from './demo-analytics-interface';
import { DemoSnapshotAdapter } from './demo-snapshot-adapter';

const DemoAdapterContext = createContext<DemoAnalyticsAdapter>(DemoSnapshotAdapter);

export function DemoAdapterProvider({ adapter, children }: { adapter: DemoAnalyticsAdapter; children: ReactNode }) {
  return <DemoAdapterContext.Provider value={adapter}>{children}</DemoAdapterContext.Provider>;
}

export function useDemoAdapter(): DemoAnalyticsAdapter {
  return useContext(DemoAdapterContext);
}
