import { createContext, useContext, type ReactNode } from 'react';
import type { FrontendDataAdapter } from './interface';
import { MockAdapter } from './mock-adapter';

const AdapterContext = createContext<FrontendDataAdapter>(MockAdapter);

export function AdapterProvider({ adapter, children }: { adapter: FrontendDataAdapter; children: ReactNode }) {
  return <AdapterContext.Provider value={adapter}>{children}</AdapterContext.Provider>;
}

export function useAdapter(): FrontendDataAdapter {
  return useContext(AdapterContext);
}
