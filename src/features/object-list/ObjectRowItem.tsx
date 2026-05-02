import { memo, type Ref, useMemo, useRef, useState } from 'react';
import {
  Component,
  Flame,
  Droplets,
  Zap,
  Wind,
  Snowflake,
  Leaf,
  Mountain,
  Star,
  Sword,
  Users,
  Pin,
  PowerOff,
  AlertTriangle,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { WorkspaceObjectNode } from '../../types/workspace';
import { cn, getFileUrl } from '../../lib/utils';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useThumbnail } from '../../hooks/useThumbnail';
import { buildWorkspaceSwitchPolicy } from '../workspace-runtime/actions/workspaceSwitchPolicy';
import { WorkspaceSwitchLabel } from '../workspace-runtime/components/WorkspaceSwitchLabel';

/** Element icon map for metadata display */
const ELEMENT_ICONS: Record<string, typeof Flame> = {
  Pyro: Flame,
  Hydro: Droplets,
  Electro: Zap,
  Anemo: Wind,
  Cryo: Snowflake,
  Dendro: Leaf,
  Geo: Mountain,
};

interface ParsedMeta {
  element?: string;
  weapon_type?: string;
  rarity?: number;
  gender?: string;
  path?: string;
}

function parseMetadata(raw: string | undefined): ParsedMeta {
  if (!raw) return {};
  try {
    return JSON.parse(raw) as ParsedMeta;
  } catch {
    return {};
  }
}

function formatObjectModCount(enabledCount: number, modCount: number): string {
  return `${enabledCount}/${modCount}`;
}

interface ObjectRowItemProps extends React.HTMLAttributes<HTMLDivElement> {
  obj: WorkspaceObjectNode;
  isSelected: boolean;
  isMobile: boolean;
  onClick: () => void;
  ref?: Ref<HTMLDivElement>;
  /** When true, show a "Move to {name}" overlay for DnD */
  isDropTarget?: boolean;
  /** Bulk-select state */
  isBulkSelected?: boolean;
  onToggleBulkSelect?: (id: string, ctrl: boolean, shift: boolean) => void;
}

function ObjectRowItemInner({
  obj,
  isSelected,
  isMobile,
  onClick,
  ref,
  className,
  isDropTarget,
  isBulkSelected = false,
  onToggleBulkSelect,
  ...rest
}: ObjectRowItemProps) {
  const { t } = useTranslation(['objects', 'common']);
  const { activeGame } = useActiveGame();

  // Dynamic thumbnail: resolve from physical folder (same as FolderGrid)
  const absFolderPath =
    activeGame?.mod_path && obj.folder_path ? `${activeGame.mod_path}/${obj.folder_path}` : null;
  const { data: dynamicThumb } = useThumbnail(
    activeGame?.id || '',
    absFolderPath ?? '',
    !!absFolderPath,
  );

  // Fallback chain: dynamic folder scan → DB thumbnail_path (MasterDB) → null
  const thumbnailUrl = dynamicThumb
    ? dynamicThumb
    : obj.thumbnail_path
      ? getFileUrl(obj.thumbnail_path)
      : null;

  const [imgError, setImgError] = useState(false);

  // Reset imgError when thumbnailUrl changes (e.g., after toggle/refetch)
  const prevUrlRef = useRef(thumbnailUrl);
  if (prevUrlRef.current !== thumbnailUrl) {
    prevUrlRef.current = thumbnailUrl;
    if (imgError) setImgError(false);
  }

  const meta = useMemo(() => parseMetadata(obj.metadata), [obj.metadata]);
  const switchPolicy = useMemo(() => buildWorkspaceSwitchPolicy(t, obj), [obj, t]);

  const ElementIcon = meta.element ? ELEMENT_ICONS[meta.element] : null;

  /** Object disabled is driven only by the physical object folder prefix on disk. */
  const isDisabled = obj.is_object_disabled;
  const isInactive = !obj.is_effectively_active && obj.mod_count > 0;

  return (
    <div
      ref={ref}
      {...rest}
      data-object-id={obj.id}
      role="button"
      tabIndex={0}
      className={cn(
        'group relative flex items-center gap-3 w-full px-2 py-1.5 rounded-lg transition-all duration-200 border select-none outline-none focus-visible:ring-1 focus-visible:ring-primary/50',
        obj.has_naming_conflict ? 'border-warning/50 ring-1 ring-warning/30' : 'border-transparent',
        isSelected
          ? 'bg-primary/10 border-primary/20 shadow-sm'
          : isDropTarget
            ? 'bg-primary/5 border-primary/30 ring-1 ring-primary/40'
            : 'hover:bg-base-200/50 hover:border-base-300/30',
        isInactive && !isSelected && 'bg-base-200/25',
        isMobile ? 'px-3 py-2' : 'px-2 py-1.5',
        className,
      )}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClick();
        }
      }}
    >
      {/* Thumbnail or Checkbox (replaces thumbnail on hover/check) */}
      <div
        className={cn(
          'relative shrink-0 overflow-hidden rounded-xl bg-base-300 flex items-center justify-center border border-base-content/5 transition-all',
          isMobile ? 'w-16 h-16' : 'w-14 h-14',
          isSelected && 'border-primary/20 bg-base-100',
          isBulkSelected && 'bg-primary/20 border-primary',
        )}
      >
        {/* Avatar Layer */}
        <div
          className={cn(
            'absolute inset-0 flex items-center justify-center transition-opacity duration-200',
            isBulkSelected ? 'opacity-0' : 'opacity-100 group-hover:opacity-0',
          )}
        >
          {thumbnailUrl && !imgError ? (
            <img
              src={thumbnailUrl}
              alt={obj.name}
              className={cn(
                'w-full h-full object-cover transition-transform duration-300 group-hover:scale-110',
                isDisabled && 'grayscale brightness-75 opacity-90',
              )}
              loading="lazy"
              onError={() => setImgError(true)}
            />
          ) : (
            <Component
              size={isMobile ? 22 : 20}
              className={cn(
                'text-base-content/30 transition-colors',
                isSelected && 'text-primary/60',
                isDisabled && 'grayscale',
              )}
            />
          )}
          {/* Power-off overlay when fully disabled */}
          {isDisabled && (
            <div
              data-testid="power-off-overlay"
              className="absolute inset-0 flex items-center justify-center bg-overlay-mask rounded-xl pointer-events-none"
            >
              <PowerOff size={isMobile ? 18 : 15} className="text-base-content/75 drop-shadow" />
            </div>
          )}
          {/* Naming Conflict overlay (both X and DISABLED X exist) */}
          {obj.has_naming_conflict && (
            <div
              className="absolute inset-0 flex items-center justify-center bg-warning/20 rounded-xl pointer-events-none"
              title={t('item.conflict_tooltip')}
            >
              <AlertTriangle size={isMobile ? 16 : 13} className="text-warning drop-shadow" />
            </div>
          )}
        </div>

        {/* Checkbox Layer */}
        {onToggleBulkSelect && (
          <div
            className={cn(
              'absolute inset-0 flex items-center justify-center transition-opacity duration-200',
              isBulkSelected ? 'opacity-100' : 'opacity-0 group-hover:opacity-100',
            )}
          >
            <input
              type="checkbox"
              className="checkbox checkbox-primary"
              checked={isBulkSelected}
              onChange={(e) => {
                e.stopPropagation();
                const isShift =
                  e.nativeEvent instanceof MouseEvent && (e.nativeEvent as MouseEvent).shiftKey;
                // Always toggle as ctrl-click so checking/unchecking doesn't clear others
                onToggleBulkSelect(obj.id, true, isShift);
              }}
              onClick={(e) => e.stopPropagation()}
            />
          </div>
        )}
      </div>

      <div className="flex-1 min-w-0 flex flex-col gap-0.5">
        {/* Name row */}
        <div className="flex items-center justify-between gap-2">
          <span
            className={cn(
              'font-medium truncate text-sm transition-colors',
              isSelected ? 'text-primary' : 'text-base-content/90 group-hover:text-base-content',
              isDisabled && 'line-through text-base-content/40',
              isInactive && !isDisabled && 'text-base-content/55',
            )}
          >
            {obj.name}
          </span>

          <div className="flex items-center gap-1.5">
            {obj.is_pinned ? <Pin size={12} className="text-secondary rotate-45" /> : null}
            {obj.mod_count > 0 ? (
              <span
                className={cn(
                  'badge badge-xs font-mono tabular-nums border-0',
                  isInactive
                    ? 'bg-base-300/35 text-base-content/30'
                    : 'bg-base-300/50 text-base-content/40',
                )}
              >
                {formatObjectModCount(obj.enabled_count, obj.mod_count)}
              </span>
            ) : null}
          </div>
        </div>

        {obj.matched_alias_name && obj.matched_alias_name !== obj.name ? (
          <div className="text-[11px] text-base-content/45 truncate">
            {t('item.matched_alias', { alias: obj.matched_alias_name })}
          </div>
        ) : null}

        {/* Metadata subtext — element icon, weapon, rarity, or fallback to mod count */}
        <div className="relative flex items-center min-h-4 w-full text-xs text-base-content/40">
          {isDisabled && (
            <div className="absolute inset-y-0 left-0 flex items-center opacity-0 group-hover:opacity-100 transition-opacity duration-200 z-10 pointer-events-none">
              <span className="font-bold text-[10px] text-error flex items-center gap-1 drop-shadow-sm uppercase tracking-wider">
                <PowerOff size={10} /> {t('item.disabled_overlay')}
              </span>
            </div>
          )}

          <div
            className={cn(
              'flex items-center gap-1.5 transition-opacity duration-200',
              isDisabled && 'group-hover:opacity-0',
              isInactive && !isDisabled && 'text-base-content/30',
            )}
          >
            {ElementIcon && <ElementIcon size={11} className="shrink-0 text-base-content/50" />}
            {meta.weapon_type && (
              <>
                <Sword size={10} className="shrink-0 text-base-content/30" />
                <span className="truncate text-[11px]">{meta.weapon_type}</span>
              </>
            )}
            <WorkspaceSwitchLabel
              node={obj}
              policy={switchPolicy}
              className="truncate text-[11px]"
            />
            {meta.path && !meta.weapon_type && !switchPolicy.label && (
              <span className="truncate text-[11px]">{meta.path}</span>
            )}
            {meta.rarity ? (
              <span className="flex items-center gap-0.5 text-warning/70">
                <Star size={10} className="fill-current" />
                <span className="text-[10px] tabular-nums">{meta.rarity}</span>
              </span>
            ) : null}
            {meta.gender && (
              <span className="flex items-center gap-0.5">
                <Users size={10} className="text-base-content/30" />
                <span className="text-[10px]">{meta.gender}</span>
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Selection indicator bar */}
      {isSelected && !isBulkSelected && (
        <div className="absolute right-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-primary rounded-l-full shadow-[0_0_10px_rgba(var(--p),0.5)]" />
      )}
    </div>
  );
}

/**
 * Fix 2: React.memo with custom comparator.
 * Without this, toggling one item causes ALL rows to re-render.
 * We compare only fields that affect visual output.
 */
export default memo(
  ObjectRowItemInner,
  (prev, next) =>
    prev.obj.id === next.obj.id &&
    prev.obj.name === next.obj.name &&
    prev.obj.folder_path === next.obj.folder_path &&
    prev.obj.matched_alias_name === next.obj.matched_alias_name &&
    prev.obj.status === next.obj.status &&
    prev.obj.mod_count === next.obj.mod_count &&
    prev.obj.enabled_count === next.obj.enabled_count &&
    prev.obj.is_pinned === next.obj.is_pinned &&
    prev.obj.is_object_disabled === next.obj.is_object_disabled &&
    prev.obj.has_naming_conflict === next.obj.has_naming_conflict &&
    prev.obj.thumbnail_path === next.obj.thumbnail_path &&
    prev.obj.metadata === next.obj.metadata &&
    prev.isMobile === next.isMobile &&
    prev.isSelected === next.isSelected &&
    prev.isDropTarget === next.isDropTarget &&
    prev.isBulkSelected === next.isBulkSelected &&
    prev.className === next.className,
);
