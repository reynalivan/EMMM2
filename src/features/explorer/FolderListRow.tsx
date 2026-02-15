import { Folder, File } from 'lucide-react';

interface FolderListRowProps {
  item: {
    id: string;
    name: string;
    type: string;
    enabled: boolean;
    imageUrl: string | null;
  };
  isSelected: boolean;
  toggleSelection: (id: string, multi: boolean) => void;
  clearSelection: () => void;
}

export default function FolderListRow({
  item,
  isSelected,
  toggleSelection,
  clearSelection,
}: FolderListRowProps) {
  return (
    <div
      id={`grid-item-${item.id}`}
      onClick={(e) => {
        if (!e.ctrlKey && !e.shiftKey) clearSelection();
        toggleSelection(item.id, e.ctrlKey || e.shiftKey);
      }}
      className={`
        flex items-center gap-3 p-2 rounded-md border cursor-pointer active:scale-[0.98] transition-all
        ${isSelected ? 'border-primary/50 bg-primary/10' : 'border-base-content/5 bg-base-200'}
      `}
    >
      <div className="w-24 h-20 shrink-0 bg-base-300 rounded-md overflow-hidden flex items-center justify-center select-none border border-base-content/5">
        {item.imageUrl ? (
          <img
            src={item.imageUrl}
            alt={item.name}
            className="w-full h-full object-cover opacity-90"
            draggable={false}
          />
        ) : item.type === 'folder' ? (
          <Folder size={24} className="text-base-content/20" />
        ) : (
          <File size={24} className="text-base-content/20" />
        )}
      </div>

      <div className="min-w-0 flex-1">
        <div
          className={`text-sm font-medium truncate leading-tight ${isSelected ? 'text-white' : 'text-base-content/80'}`}
        >
          {item.name}
        </div>
        <div className="flex items-center gap-2 mt-2">
          <span
            className={`badge badge-xs border-0 font-bold ${item.enabled ? 'bg-secondary/20 text-secondary' : 'bg-base-content/10 text-base-content/30'}`}
          >
            {item.enabled ? 'ON' : 'OFF'}
          </span>
          {item.type === 'folder' && (
            <span className="text-[9px] text-base-content/30 uppercase tracking-widest font-bold">
              DIR
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
