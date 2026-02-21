import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { AdapterProvider } from './adapters/context';
import { MockAdapter } from './adapters/mock-adapter';
import { SnapshotFrontendAdapter } from './adapters/snapshot-frontend-adapter';
import App from './app/App';
import './styles/app.css';

const useMock = import.meta.env.VITE_USE_MOCK_ADAPTER === 'true';
const rootAdapter = useMock ? MockAdapter : SnapshotFrontendAdapter;

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
      <AdapterProvider adapter={rootAdapter}>
        <App />
      </AdapterProvider>
    </BrowserRouter>
  </React.StrictMode>
);
