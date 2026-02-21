import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { AdapterProvider } from './adapters/context';
import { MockAdapter } from './adapters/mock-adapter';
import App from './app/App';
import './styles/app.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
      <AdapterProvider adapter={MockAdapter}>
        <App />
      </AdapterProvider>
    </BrowserRouter>
  </React.StrictMode>
);
