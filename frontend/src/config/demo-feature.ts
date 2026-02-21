/** Enable "Use my Dune key" mode on the demo page. Key is sent only to proxy, never stored. */
export const DEMO_BYOK_ENABLED = import.meta.env.VITE_DEMO_BYOK_ENABLED === 'true';

/** Base URL for demo proxy when using BYOK or proxy mode. Empty = same origin (rely on Vite proxy). */
export const DEMO_PROXY_BASE = import.meta.env.VITE_DEMO_PROXY_URL || '';
