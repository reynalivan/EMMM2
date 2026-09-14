import type { ComponentPropsWithoutRef, ReactNode } from 'react';
import { cn } from '@/shared/lib/utils';
import { LiquidSurface } from '@/shared/ui/liquid';

export type WorkspacePageDensity = 'wide' | 'form';

const densityClasses: Record<WorkspacePageDensity, string> = {
  wide: 'max-w-7xl',
  form: 'max-w-3xl',
};

interface WorkspacePageFrameProps extends Omit<ComponentPropsWithoutRef<'div'>, 'children'> {
  children: ReactNode;
  context?: ReactNode;
}

interface WorkspacePageContentProps {
  children: ReactNode;
  density?: WorkspacePageDensity;
  className?: string;
}

interface WorkspaceContextBarProps {
  description?: ReactNode;
  tools?: ReactNode;
  actions?: ReactNode;
  density?: WorkspacePageDensity;
  className?: string;
}

/** Shared shell for workspace pages. The global top bar remains the page title owner. */
export function WorkspacePageFrame({
  children,
  context,
  className,
  ...props
}: WorkspacePageFrameProps) {
  return (
    <div
      {...props}
      data-has-context={context ? 'true' : 'false'}
      className={cn(
        'workspace-page-frame relative flex h-full min-h-0 flex-col overflow-hidden bg-base-100/85',
        className,
      )}
    >
      {context && (
        <div className="shrink-0 lg:absolute lg:inset-x-0 lg:top-0 lg:z-20">{context}</div>
      )}
      {children}
    </div>
  );
}

export function WorkspacePageContent({
  children,
  density = 'wide',
  className,
}: WorkspacePageContentProps) {
  return (
    <div className="workspace-scroll-owner min-h-0 flex-1 overflow-auto">
      <div
        className={cn(
          'mx-auto min-h-full w-full p-4 pt-[calc(var(--workspace-topbar-height)+var(--workspace-context-chrome-height)+1rem)] sm:p-6 sm:pt-[calc(var(--workspace-topbar-height)+var(--workspace-context-chrome-height)+1.5rem)]',
          densityClasses[density],
          className,
        )}
      >
        {children}
      </div>
    </div>
  );
}

export function WorkspaceContextBar({
  description,
  tools,
  actions,
  density = 'wide',
  className,
}: WorkspaceContextBarProps) {
  if (!description && !tools && !actions) return null;

  return (
    <LiquidSurface
      liquidRole="nav"
      className={cn('workspace-context-bar block w-full shrink-0', className)}
      contentClassName="h-auto"
    >
      <div
        className={cn(
          'mx-auto flex min-h-14 w-full flex-col gap-3 px-4 pb-3 pt-[calc(var(--workspace-topbar-height)+0.75rem)] sm:flex-row sm:items-center sm:justify-between sm:px-6 lg:min-h-16 lg:pb-3 lg:pt-[calc(var(--workspace-topbar-height)+0.75rem)]',
          densityClasses[density],
        )}
      >
        {description && <div className="min-w-0 text-sm text-base-content/65">{description}</div>}
        {(tools || actions) && (
          <div className="flex min-w-0 flex-wrap items-center gap-2 sm:justify-end">
            {tools}
            {actions}
          </div>
        )}
      </div>
    </LiquidSurface>
  );
}
