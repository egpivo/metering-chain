import type { ReactNode } from 'react';

export interface Column<T> {
  key: string;
  label: string;
  width?: string;
  render?: (row: T) => ReactNode;
}

interface DataTableProps<T> {
  columns: Column<T>[];
  data: T[];
  keyFn: (row: T) => string;
  emptyMessage?: string;
  tableClassName?: string;
}

export function DataTable<T>({ columns, data, keyFn, emptyMessage = 'No data', tableClassName }: DataTableProps<T>) {
  if (data.length === 0) {
    return <p className="color-text-muted" style={{ color: 'var(--color-text-muted)' }}>{emptyMessage}</p>;
  }
  const hasWidths = columns.some((c) => c.width);
  const tableClass = tableClassName ? `data-table ${tableClassName}` : 'data-table';
  return (
    <table className={tableClass}>
      {hasWidths && (
        <colgroup>
          {columns.map((c) => (
            <col key={c.key} style={c.width ? { width: c.width } : undefined} />
          ))}
        </colgroup>
      )}
      <thead>
        <tr>
          {columns.map((c) => (
            <th key={c.key}>{c.label}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {data.map((row) => (
          <tr key={keyFn(row)}>
            {columns.map((col) => (
              <td key={col.key}>{col.render ? col.render(row) : (row as Record<string, unknown>)[col.key] as ReactNode}</td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
