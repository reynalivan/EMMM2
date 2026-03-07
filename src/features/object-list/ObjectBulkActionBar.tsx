import {
  Trash2,
  Pin,
  PinOff,
  Power,
  PowerOff,
  TagIcon,
  Tags,
  X,
  MoreHorizontal,
  Sparkles,
} from 'lucide-react';

interface ObjectBulkActionBarProps {
  count: number;
  onDelete: () => void;
  onPin: (pin: boolean) => void;
  onEnable: () => void;
  onDisable: () => void;
  onAddTags: () => void;
  onRemoveTags: () => void;
  onAutoOrganize: () => void;
  onClear: () => void;
}

export default function ObjectBulkActionBar({
  count,
  onDelete,
  onPin,
  onEnable,
  onDisable,
  onAddTags,
  onRemoveTags,
  onAutoOrganize,
  onClear,
}: ObjectBulkActionBarProps) {
  if (count === 0) return null;

  return (
    <div className="flex items-center justify-between w-full h-8 px-2 bg-primary text-primary-content rounded-md shadow-sm animate-in fade-in zoom-in-95 duration-200">
      {/* Left: Clear + Count */}
      <div className="flex items-center gap-2">
        <button
          className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={onClear}
          title="Clear selection"
        >
          <X size={15} />
        </button>
        <span className="text-xs font-semibold tabular-nums">{count} selected</span>
      </div>

      {/* Right: Primary actions + More dropdown */}
      <div className="flex items-center gap-1">
        <button
          className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={onDelete}
          title="Delete selected"
        >
          <Trash2 size={15} />
        </button>

        <button
          className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
          onClick={() => onPin(true)}
          title="Pin selected"
        >
          <Pin size={15} />
        </button>

        {/* Dropdown for secondary actions */}
        <div className="dropdown dropdown-end">
          <div
            tabIndex={0}
            role="button"
            className="btn btn-xs btn-ghost btn-circle text-primary-content hover:bg-primary-content/20"
            title="More actions"
          >
            <MoreHorizontal size={15} />
          </div>
          <ul
            tabIndex={0}
            className="dropdown-content z-50 menu p-2 shadow bg-base-200 text-base-content rounded-box w-40 mt-1"
          >
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5"
                onClick={() => onPin(false)}
              >
                <PinOff size={14} className="opacity-70" />
                Unpin
              </button>
            </li>
            <div className="divider my-0.5"></div>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-success"
                onClick={onEnable}
              >
                <Power size={14} className="opacity-70" />
                Enable
              </button>
            </li>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-warning"
                onClick={onDisable}
              >
                <PowerOff size={14} className="opacity-70" />
                Disable
              </button>
            </li>
            <div className="divider my-0.5"></div>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-info"
                onClick={onAutoOrganize}
              >
                <Sparkles size={14} className="opacity-70" />
                Auto Organize
              </button>
            </li>
            <div className="divider my-0.5"></div>
            <li>
              <button className="flex items-center gap-2 text-xs py-1.5" onClick={onAddTags}>
                <TagIcon size={14} className="opacity-70" />
                Add Tags
              </button>
            </li>
            <li>
              <button
                className="flex items-center gap-2 text-xs py-1.5 text-error"
                onClick={onRemoveTags}
              >
                <Tags size={14} className="opacity-70" />
                Remove Tags
              </button>
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}
