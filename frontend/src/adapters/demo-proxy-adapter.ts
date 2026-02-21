/**
 * Demo adapter that fetches from the backend proxy (Day 6+7).
 * Day 8: optional getApiKey for BYOK — key sent only to proxy (header), never stored.
 */

import type { DemoAnalyticsAdapter } from './demo-analytics-interface';
import type { DemoWindowAggregate } from '../domain/types';

const DEFAULT_BASE = '';
const BYOK_HEADER = 'X-Dune-Api-Key';

function buildQuery(params: Parameters<DemoAnalyticsAdapter['getDemoWindows']>[0]): string {
  const q = new URLSearchParams();
  q.set('start_date', params.start_date);
  q.set('end_date', params.end_date);
  if (params.owner) q.set('owner', params.owner);
  if (params.service_id) q.set('service_id', params.service_id);
  q.set('window_granularity', params.window_granularity);
  if (params.operator_share_bps != null) q.set('operator_share_bps', String(params.operator_share_bps));
  if (params.protocol_fee_bps != null) q.set('protocol_fee_bps', String(params.protocol_fee_bps));
  if (params.reserve_bps != null) q.set('reserve_bps', String(params.reserve_bps));
  if (params.top_n != null) q.set('top_n', String(params.top_n));
  return q.toString();
}

export interface DemoProxyAdapterOptions {
  /** Session-only; key sent to proxy in header, never persisted. */
  getApiKey?: () => string | null;
}

export function createDemoProxyAdapter(
  baseUrl: string = DEFAULT_BASE,
  options?: DemoProxyAdapterOptions
): DemoAnalyticsAdapter {
  return {
    async getDemoWindows(params) {
      const url = `${baseUrl}/api/demo/windows?${buildQuery(params)}`;
      const headers: Record<string, string> = {};
      const key = options?.getApiKey?.()?.trim();
      if (key) headers[BYOK_HEADER] = key;
      const res = await fetch(url, { headers });
      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: res.statusText }));
        throw new Error((err as { error?: string }).error || `Proxy ${res.status}`);
      }
      const data = (await res.json()) as { windows: DemoWindowAggregate[] };
      return data.windows ?? [];
    },

    async getDemoEvidence(_window_id: string, _owner: string, _service_id: string) {
      return Promise.resolve(null);
    },
  };
}
