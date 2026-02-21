import type { ApiErrorView } from '../domain/types';

interface ErrorBannerProps {
  error: ApiErrorView;
  onDismiss?: () => void;
}

export function ErrorBanner({ error, onDismiss }: ErrorBannerProps) {
  return (
    <div className="error-banner" role="alert">
      <div className="error-code">{error.error_code}</div>
      <p style={{ margin: 'var(--space-1) 0 0' }}>{error.message}</p>
      {error.suggested_action && (
        <p style={{ margin: 'var(--space-2) 0 0', color: 'var(--color-text-muted)' }}>→ {error.suggested_action}</p>
      )}
      {onDismiss && (
        <button type="button" onClick={onDismiss} style={{ marginTop: 'var(--space-2)' }}>
          Dismiss
        </button>
      )}
    </div>
  );
}
