import type { ReactNode } from 'react';

export interface ActionItem {
  label: string;
  onClick: () => void;
  variant?: 'primary' | 'danger' | 'default';
  disabled?: boolean;
}

interface ActionPanelProps {
  actions: ActionItem[];
  title?: string;
  children?: ReactNode;
}

export function ActionPanel({ actions, title, children }: ActionPanelProps) {
  return (
    <div className="card">
      {title && <h3>{title}</h3>}
      {children}
      <div style={{ display: 'flex', gap: 'var(--space-2)', flexWrap: 'wrap', marginTop: 'var(--space-3)' }}>
        {actions.map((a, i) => (
          <button
            key={i}
            type="button"
            className={a.variant === 'primary' ? 'primary' : a.variant === 'danger' ? 'danger' : ''}
            disabled={a.disabled}
            onClick={a.onClick}
          >
            {a.label}
          </button>
        ))}
      </div>
    </div>
  );
}
