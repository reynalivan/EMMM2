/**
 * Epic 3: ObjectRow — renders a single object in the sidebar list.
 * Uses real ObjectSummary data instead of mock data.
 * Shows name, initials, mod count, and enabled indicator.
 */

import { Hash, Package } from 'lucide-react';
import type { ObjectSummary } from '../../types/object';

interface ObjectRowProps {
  obj: ObjectSummary;
  isSelected: boolean;
  isMobile: boolean;
  onClick: () => void;
  className?: string;
}

/** Generate a deterministic gradient from an object name */
function getGradient(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;
  return `linear-gradient(135deg, hsl(${hue}, 60%, 25%), hsl(${(hue + 40) % 360}, 50%, 20%))`;
}

export default function ObjectRow({
  obj,
  isSelected,
  isMobile,
  onClick,
  className = '',
}: ObjectRowProps) {
  return (
    <div
      onClick={onClick}
      className={`
        flex items-center gap-2.5 px-2.5 rounded-md cursor-pointer transition-all duration-100 group select-none
        ${
          isSelected
            ? 'bg-base-content/10 text-white shadow-sm'
            : 'text-base-content/60 hover:bg-base-300/60 hover:text-base-content/80'
        }
        ${className}
      `}
      style={{ height: isMobile ? '52px' : '40px' }}
    >
      {/* Avatar / Initials */}
      <div
        className={`
          w-8 h-8 rounded-[10px] flex items-center justify-center text-[10px] font-bold transition-all
          ${isSelected ? 'bg-primary text-white' : 'bg-base-300 text-base-content/40 group-hover:bg-base-300/80'}
        `}
        style={!isSelected ? { background: getGradient(obj.name) } : {}}
      >
        {isSelected ? (
          <Hash size={14} />
        ) : obj.thumbnail_path ? (
          <img
            src={obj.thumbnail_path}
            alt={obj.name}
            className="w-full h-full rounded-[10px] object-cover"
          />
        ) : (
          obj.name.substring(0, 2).toUpperCase()
        )}
      </div>

      {/* Name + mod count */}
      <div className="flex-1 min-w-0 flex flex-col justify-center">
        <div
          className={`font-medium ${isMobile ? 'text-sm' : 'text-[14px]'} truncate leading-tight`}
        >
          {obj.name}
        </div>
        {obj.mod_count > 0 && (
          <div className="flex items-center gap-1 text-[10px] text-base-content/40 leading-tight">
            <Package size={8} />
            <span>
              {obj.enabled_count}/{obj.mod_count} mods
            </span>
          </div>
        )}
      </div>

      {/* Enabled indicator */}
      {obj.enabled_count > 0 && (
        <div
          className={`w-1.5 h-1.5 rounded-full transition-colors ${isSelected ? 'bg-primary animate-pulse' : 'bg-success/60 group-hover:bg-success'}`}
        />
      )}
    </div>
  );
}
