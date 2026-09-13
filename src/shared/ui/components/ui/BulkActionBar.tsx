import { Fragment } from 'react';
import { MoreHorizontal, Power, PowerOff, ShieldAlert, ShieldCheck, X } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { LiquidSurface } from '@/shared/ui/liquid';

export interface BulkBarAction {
  icon: LucideIcon;
  label: string;
  onClick: () => void;
  className?: string;
  dividerBefore?: boolean;
}

interface BulkActionBarLabels {
  clear: string;
  /** Floating: caption under the count. Inline: full "{count} selected" text. */
  count: string;
  safe: string;
  unsafe: string;
  more: string;
  menuTitle?: string;
}

interface BulkActionBarProps {
  /** floating: folder-grid pill fixed above the grid. inline: object-list toolbar strip. */
  variant: 'floating' | 'inline';
  count: number;
  onClear: () => void;
  onMarkSafe: (safe: boolean) => void;
  labels: BulkActionBarLabels;
  /** Circular icon buttons rendered before the safety toggle. */
  iconActions: BulkBarAction[];
  /** Enable/disable join group (floating variant only). */
  toggleGroup?: {
    tooltip: string;
    enableLabel: string;
    disableLabel: string;
    onToggle: (enable: boolean) => void;
  };
  dropdownActions: BulkBarAction[];
  mutationsDisabled?: boolean;
}

/**
 * BulkActionBar — shared selection action bar for folder-grid and object-list.
 * Owns the manual Safe/Unsafe classification actions; per-feature actions come in
 * as lists, layout/sizing is driven by the variant.
 */
export default function BulkActionBar({
  variant,
  count,
  onClear,
  onMarkSafe,
  labels,
  iconActions,
  toggleGroup,
  dropdownActions,
  mutationsDisabled = false,
}: BulkActionBarProps) {
  if (count === 0) return null;

  const isFloating = variant === 'floating';
  const btnSize = isFloating ? 'btn-sm' : 'btn-xs';
  const iconSize = isFloating ? 18 : 15;
  const itemIconSize = isFloating ? 16 : 14;
  const itemClass = isFloating
    ? 'bulk-action-bar__menu-item flex min-h-9 w-full items-center gap-2 whitespace-nowrap py-2'
    : 'bulk-action-bar__menu-item flex w-full items-center gap-2 whitespace-nowrap py-1.5 text-xs';
  const circleBtn = `btn ${btnSize} btn-ghost btn-circle text-base-content/65 hover:bg-base-200 hover:text-base-content focus-visible:outline focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2 active:translate-y-px`;
  const compactDropdownActions: BulkBarAction[] = [];

  if (toggleGroup) {
    compactDropdownActions.push(
      {
        icon: Power,
        label: toggleGroup.enableLabel,
        onClick: () => toggleGroup.onToggle(true),
      },
      {
        icon: PowerOff,
        label: toggleGroup.disableLabel,
        onClick: () => toggleGroup.onToggle(false),
      },
    );
  }

  compactDropdownActions.push(
    ...iconActions,
    { icon: ShieldCheck, label: labels.safe, onClick: () => onMarkSafe(true) },
    { icon: ShieldAlert, label: labels.unsafe, onClick: () => onMarkSafe(false) },
    ...dropdownActions.map((action, index) => ({
      ...action,
      dividerBefore: index === 0 || action.dividerBefore,
    })),
  );

  const safetyButtons = (
    <div className="join">
      <button
        className={`btn ${btnSize} join-item btn-ghost text-base-content/65 hover:text-success`}
        onClick={() => onMarkSafe(true)}
        title={labels.safe}
        aria-label={labels.safe}
        disabled={mutationsDisabled}
      >
        <ShieldCheck size={iconSize} />
      </button>
      <button
        className={`btn ${btnSize} join-item btn-ghost text-base-content/65 hover:text-warning`}
        onClick={() => onMarkSafe(false)}
        title={labels.unsafe}
        aria-label={labels.unsafe}
        disabled={mutationsDisabled}
      >
        <ShieldAlert size={iconSize} />
      </button>
    </div>
  );

  const dropdown = (
    <div
      className={`dropdown dropdown-end relative ${
        isFloating ? 'z-[calc(var(--workspace-layer-overlay)+1)] dropdown-top' : 'z-[90]'
      }`}
    >
      <button
        type="button"
        tabIndex={0}
        className={circleBtn}
        title={labels.more}
        aria-label={labels.more}
        disabled={mutationsDisabled}
      >
        <MoreHorizontal size={iconSize} />
      </button>
      <LiquidSurface
        liquidRole="overlay"
        className={
          isFloating
            ? 'dropdown-content z-[var(--workspace-layer-popover)] mb-3 w-60 max-w-[calc(100cqw-2rem)] rounded-xl text-base-content shadow-lg'
            : 'object-bulk-action-menu dropdown-content z-[var(--workspace-layer-popover)] mt-2 w-56 max-w-[calc(100vw-1rem)] rounded-xl text-base-content shadow-lg'
        }
      >
        <ul tabIndex={0} className="menu w-full whitespace-nowrap p-2">
          {labels.menuTitle && (
            <li className="menu-title px-4 py-1.5 text-[10px] uppercase font-bold text-muted">
              {labels.menuTitle}
            </li>
          )}
          {compactDropdownActions.map((action, index) => (
            <Fragment key={action.label}>
              {action.dividerBefore && (
                <div className={isFloating ? 'divider my-0 opacity-50' : 'divider my-0.5'}></div>
              )}
              <li>
                <button
                  className={`${itemClass} ${
                    index < compactDropdownActions.length - dropdownActions.length
                      ? 'bulk-action-bar__compact-only'
                      : ''
                  } ${action.className ?? ''}`}
                  onClick={action.onClick}
                  disabled={mutationsDisabled}
                >
                  <action.icon size={itemIconSize} className="opacity-70" />
                  {action.label}
                </button>
              </li>
            </Fragment>
          ))}
        </ul>
      </LiquidSurface>
    </div>
  );

  return (
    <div
      className={
        isFloating
          ? 'bulk-action-bar bulk-action-bar--floating absolute bottom-4 left-1/2 z-[var(--workspace-layer-overlay)] inline-flex h-12 min-w-[320px] max-w-[calc(100cqw-2rem)] -translate-x-1/2 text-base-content'
          : 'bulk-action-bar bulk-action-bar--inline relative z-[90] h-8 w-full rounded-lg border border-base-content/10 bg-base-200/90 text-base-content shadow-sm'
      }
    >
      {isFloating && (
        <LiquidSurface
          liquidRole="overlay"
          className="pointer-events-none absolute inset-0 rounded-xl shadow-lg"
          aria-hidden="true"
        >
          <span />
        </LiquidSurface>
      )}

      <div
        className={`relative z-10 flex h-full w-full items-center justify-between ${isFloating ? 'px-4' : 'px-2'}`}
      >
        {/* Left: Clear + Count */}
        <div className={`flex items-center ${isFloating ? 'gap-3' : 'gap-2'}`}>
          <button
            className={circleBtn}
            onClick={onClear}
            title={labels.clear}
            aria-label={labels.clear}
          >
            <X size={iconSize} />
          </button>
          {isFloating ? (
            <div className="flex flex-col leading-none">
              <span className="text-sm font-bold tabular-nums">{count}</span>
              <span className="text-[10px] uppercase tracking-wider opacity-70 font-semibold">
                {labels.count}
              </span>
            </div>
          ) : (
            <span className="text-xs font-semibold tabular-nums">{labels.count}</span>
          )}
        </div>

        {isFloating && (
          <div className="bulk-action-bar__divider mx-4 h-6 w-px bg-base-content/10" />
        )}

        {/* Main actions */}
        <div
          className={`bulk-action-bar__primary flex items-center ${isFloating ? 'gap-2' : 'gap-1'}`}
        >
          {toggleGroup && (
            <div className="tooltip tooltip-top" data-tip={toggleGroup.tooltip}>
              <div className="join rounded-lg border border-base-content/10 bg-base-200/70 p-0.5">
                <button
                  className="btn btn-xs join-item btn-ghost h-7 gap-1.5 border-none px-2.5 text-base-content/75 hover:text-success"
                  onClick={() => toggleGroup.onToggle(true)}
                  disabled={mutationsDisabled}
                >
                  <Power size={14} className="mr-1" />
                  {toggleGroup.enableLabel}
                </button>
                <div className="h-4 w-px self-center bg-base-content/10" />
                <button
                  className="btn btn-xs join-item btn-ghost h-7 gap-1.5 border-none px-2.5 text-base-content/75 hover:text-warning"
                  onClick={() => toggleGroup.onToggle(false)}
                  disabled={mutationsDisabled}
                >
                  <PowerOff size={14} className="mr-1" />
                  {toggleGroup.disableLabel}
                </button>
              </div>
            </div>
          )}

          {iconActions.map((action) => (
            <button
              key={action.label}
              className={`${circleBtn} ${action.className ?? ''}`}
              onClick={action.onClick}
              title={action.label}
              disabled={mutationsDisabled}
            >
              <action.icon size={iconSize} />
            </button>
          ))}

          {safetyButtons}
        </div>

        {!isFloating && <div className="flex items-center gap-1">{dropdown}</div>}

        {isFloating && (
          <>
            <div className="bulk-action-bar__divider mx-4 h-6 w-px bg-base-content/10" />
            <div className="flex items-center gap-1">{dropdown}</div>
          </>
        )}
      </div>
    </div>
  );
}
