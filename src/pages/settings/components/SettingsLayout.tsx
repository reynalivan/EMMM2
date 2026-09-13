// Hallmark · pre-emit critique: P4 H5 E5 S5 R5 V4 · restrained settings ledger
import type { ReactNode } from 'react';

type SettingsSectionProps = {
  id: string;
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  children?: ReactNode;
  className?: string;
};

export function SettingsSection({
  id,
  title,
  description,
  action,
  children,
  className = '',
}: SettingsSectionProps) {
  return (
    <section
      className={`border-b border-base-300 py-5 first:pt-0 last:border-b-0 ${className}`}
      aria-labelledby={id}
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <h2 id={id} className="text-sm font-semibold text-base-content">
            {title}
          </h2>
          {description && (
            <p className="mt-1 max-w-2xl text-xs leading-5 text-base-content/60">{description}</p>
          )}
        </div>
        {action && <div className="shrink-0">{action}</div>}
      </div>
      {children && <div className="mt-3">{children}</div>}
    </section>
  );
}

type SettingsRowProps = {
  label: ReactNode;
  description?: ReactNode;
  control: ReactNode;
  className?: string;
};

export function SettingsRow({ label, description, control, className = '' }: SettingsRowProps) {
  return (
    <div
      className={`grid min-h-11 gap-3 py-2 sm:grid-cols-[minmax(0,1fr)_minmax(12rem,auto)] sm:items-center ${className}`}
    >
      <div className="min-w-0">
        <p className="text-sm font-medium text-base-content">{label}</p>
        {description && (
          <p className="mt-0.5 text-xs leading-5 text-base-content/60">{description}</p>
        )}
      </div>
      <div className="min-w-0 sm:justify-self-end">{control}</div>
    </div>
  );
}
