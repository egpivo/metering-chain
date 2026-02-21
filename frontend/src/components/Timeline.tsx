export interface TimelineStep {
  label: string;
  done: boolean;
  current?: boolean;
}

interface TimelineProps {
  steps: TimelineStep[];
}

export function Timeline({ steps }: TimelineProps) {
  return (
    <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
      {steps.map((step, i) => (
        <li
          key={i}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 'var(--space-2)',
            padding: 'var(--space-1) 0',
            color: step.done ? 'var(--color-success)' : step.current ? 'var(--color-accent)' : 'var(--color-text-muted)',
          }}
        >
          <span
            style={{
              width: 20,
              height: 20,
              borderRadius: '50%',
              border: `2px solid ${step.done ? 'var(--color-success)' : step.current ? 'var(--color-accent)' : 'var(--color-border)'}`,
              background: step.done ? 'var(--color-success)' : 'transparent',
            }}
          />
          {step.label}
        </li>
      ))}
    </ul>
  );
}
