import { Fragment } from 'react';
import { MoreHorizontal, Power, PowerOff, ShieldAlert, ShieldCheck, X } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useSafeMode } from '../../hooks/settingsQuery';

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
  more?: string;
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
 * Owns the single safeMode-adaptive safety toggle; per-feature actions come in
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
  const safeMode = useSafeMode();

  if (count === 0) return null;

  const isFloating = variant === 'floating';
  const btnSize = isFloating ? 'btn-sm' : 'btn-xs';
  const iconSize = isFloating ? 18 : 15;
  const itemIconSize = isFloating ? 16 : 14;
  const itemClass = isFloating ? 'py-2' : 'flex items-center gap-2 text-xs py-1.5';
  const circleBtn = `btn ${btnSize} btn-ghost btn-circle text-primary-content hover:bg-primary-content/20`;

  // The single safeMode-adaptive safety toggle shared by both features.
  const shieldButton = safeMode ? (
    <button
      className={`btn ${btnSize} btn-ghost btn-circle text-warning ${
        isFloating
          ? 'hover:bg-warning hover:text-warning-content border-none'
          : 'hover:bg-primary-content/20'
      }`}
      onClick={() => onMarkSafe(false)}
      title={labels.unsafe}
      disabled={mutationsDisabled}
    >
      <ShieldAlert size={iconSize} />
    </button>
  ) : (
    <button
      className={`btn ${btnSize} btn-ghost btn-circle text-success ${
        isFloating
          ? 'hover:bg-success hover:text-success-content border-none'
          : 'hover:bg-primary-content/20'
      }`}
      onClick={() => onMarkSafe(true)}
      title={labels.safe}
      disabled={mutationsDisabled}
    >
      <ShieldCheck size={iconSize} />
    </button>
  );

  const dropdown = (
    <div className={`dropdown ${isFloating ? 'dropdown-top ' : ''}dropdown-end`}>
      <div
        tabIndex={0}
        role="button"
        className={circleBtn}
        title={labels.more}
        aria-disabled={mutationsDisabled}
      >
        <MoreHorizontal size={iconSize} />
      </div>
      <ul
        tabIndex={0}
        className={
          isFloating
            ? 'dropdown-content z-60 menu p-2 shadow-xl bg-base-200 text-base-content rounded-box w-52 mb-3 border border-base-300'
            : 'dropdown-content z-50 menu p-2 shadow bg-base-200 text-base-content rounded-box w-40 mt-1'
        }
      >
        {labels.menuTitle && (
          <li className="menu-title px-4 py-1.5 text-[10px] uppercase font-bold opacity-50">
            {labels.menuTitle}
          </li>
        )}
        {dropdownActions.map((action) => (
          <Fragment key={action.label}>
            {action.dividerBefore && (
              <div className={isFloating ? 'divider my-0 opacity-50' : 'divider my-0.5'}></div>
            )}
            <li>
              <button
                className={`${itemClass} ${action.className ?? ''}`}
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
    </div>
  );

  return (
    <div
      className={
        isFloating
          ? 'fixed bottom-6 left-1/2 -translate-x-1/2 z-50 flex items-center justify-between min-w-[320px] h-12 px-4 bg-primary text-primary-content rounded-full shadow-2xl animate-in slide-in-from-bottom-4 fade-in duration-300'
          : 'flex items-center justify-between w-full h-8 px-2 bg-primary text-primary-content rounded-md shadow-sm animate-in fade-in zoom-in-95 duration-200'
      }
    >
      {/* Left: Clear + Count */}
      <div className={`flex items-center ${isFloating ? 'gap-3' : 'gap-2'}`}>
        <button className={`${circleBtn} transition-colors`} onClick={onClear} title={labels.clear}>
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

      {isFloating && <div className="h-6 w-px bg-primary-content/20 mx-4" />}

      {/* Main actions */}
      <div className={`flex items-center ${isFloating ? 'gap-2' : 'gap-1'}`}>
        {toggleGroup && (
          <div className="tooltip tooltip-top" data-tip={toggleGroup.tooltip}>
            <div className="join bg-primary-content/10 rounded-full p-0.5">
              <button
                className="btn btn-xs join-item btn-ghost text-success hover:bg-success hover:text-success-content border-none h-7 px-3"
                onClick={() => toggleGroup.onToggle(true)}
                disabled={mutationsDisabled}
              >
                <Power size={14} className="mr-1" />
                {toggleGroup.enableLabel}
              </button>
              <div className="w-px h-4 bg-primary-content/10 self-center" />
              <button
                className="btn btn-xs join-item btn-ghost text-warning hover:bg-warning hover:text-warning-content border-none h-7 px-3"
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

        {shieldButton}
        {!isFloating && dropdown}
      </div>

      {isFloating && (
        <>
          <div className="h-6 w-px bg-primary-content/20 mx-4" />
          <div className="flex items-center gap-1">{dropdown}</div>
        </>
      )}
    </div>
  );
}
