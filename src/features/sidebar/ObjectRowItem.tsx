import { type Ref, useMemo } from 'react';
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
} from 'lucide-react';
import { useState } from 'react';
import type { ObjectSummary } from '../../types/object';
import { cn } from '../../lib/utils';
import { getFileUrl } from '../../lib/utils';

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

interface ObjectRowItemProps extends React.HTMLAttributes<HTMLDivElement> {
  obj: ObjectSummary;
  isSelected: boolean;
  isMobile: boolean;
  onClick: () => void;
  ref?: Ref<HTMLDivElement>;
}

export default function ObjectRowItem({
  obj,
  isSelected,
  isMobile,
  onClick,
  ref,
  className,
  ...rest
}: ObjectRowItemProps) {
  const thumbnailUrl = obj.thumbnail_path ? getFileUrl(obj.thumbnail_path) : null;
  const [imgError, setImgError] = useState(false);

  const meta = useMemo(() => parseMetadata(obj.metadata), [obj.metadata]);

  const ElementIcon = meta.element ? ELEMENT_ICONS[meta.element] : null;

  return (
    <div
      ref={ref}
      {...rest}
      role="button"
      tabIndex={0}
      className={cn(
        'group relative flex items-center gap-3 w-full px-2 py-1.5 rounded-lg transition-all duration-200 border border-transparent select-none outline-none focus-visible:ring-1 focus-visible:ring-primary/50',
        isSelected
          ? 'bg-primary/10 border-primary/20 shadow-sm'
          : 'hover:bg-base-200/50 hover:border-base-300/30',
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
      {/* Thumbnail — 20% larger, more rounded */}
      <div
        className={cn(
          'relative shrink-0 overflow-hidden rounded-xl bg-base-300 flex items-center justify-center border border-base-content/5',
          isMobile ? 'w-16 h-16' : 'w-14 h-14',
          isSelected && 'border-primary/20 bg-base-100',
        )}
      >
        {thumbnailUrl && !imgError ? (
          <img
            src={thumbnailUrl}
            alt={obj.name}
            className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-110"
            loading="lazy"
            onError={() => setImgError(true)}
          />
        ) : (
          <Component
            size={isMobile ? 22 : 20}
            className={cn(
              'text-base-content/30 transition-colors',
              isSelected && 'text-primary/60',
            )}
          />
        )}
      </div>

      <div className="flex-1 min-w-0 flex flex-col gap-0.5">
        {/* Name row */}
        <div className="flex items-center justify-between gap-2">
          <span
            className={cn(
              'font-medium truncate text-sm transition-colors',
              isSelected ? 'text-primary' : 'text-base-content/90 group-hover:text-base-content',
            )}
          >
            {obj.name}
          </span>
          {obj.enabled_count > 0 && (
            <span className="badge badge-xs badge-primary font-mono tabular-nums bg-primary/20 text-primary border-0">
              {obj.enabled_count}
            </span>
          )}
        </div>

        {/* Metadata subtext — element icon, weapon, rarity, or fallback to mod count */}
        <div className="flex items-center gap-1.5 text-xs text-base-content/40">
          {ElementIcon && <ElementIcon size={11} className="shrink-0 text-base-content/50" />}
          {meta.weapon_type && (
            <>
              <Sword size={10} className="shrink-0 text-base-content/30" />
              <span className="truncate text-[11px]">{meta.weapon_type}</span>
            </>
          )}
          {meta.path && !meta.weapon_type && (
            <span className="truncate text-[11px]">{meta.path}</span>
          )}
          {meta.rarity && (
            <span className="flex items-center gap-0.5 text-amber-400/70">
              <Star size={10} className="fill-current" />
              <span className="text-[10px] tabular-nums">{meta.rarity}</span>
            </span>
          )}
          {meta.gender && (
            <span className="flex items-center gap-0.5">
              <Users size={10} className="text-base-content/30" />
              <span className="text-[10px]">{meta.gender}</span>
            </span>
          )}
          {/* Fallback: mod count when no meaningful metadata */}
          {!meta.element &&
            !meta.weapon_type &&
            !meta.rarity &&
            !meta.path &&
            obj.mod_count > 0 && <span className="text-[11px]">{obj.mod_count} mods</span>}
        </div>
      </div>

      {/* Selection indicator bar */}
      {isSelected && (
        <div className="absolute right-0 top-1/2 -translate-y-1/2 w-1 h-8 bg-primary rounded-l-full shadow-[0_0_10px_rgba(var(--p),0.5)]" />
      )}
    </div>
  );
}
