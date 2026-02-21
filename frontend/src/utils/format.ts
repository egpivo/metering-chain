/**
 * Number formatting for tables (readable demo/settlement figures).
 */
const intlInteger = new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 });

export function formatInteger(n: number): string {
  return intlInteger.format(n);
}

export function formatPercent(value: number, decimals = 1): string {
  return new Intl.NumberFormat('en-US', {
    style: 'percent',
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value / 100);
}
