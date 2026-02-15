import { Folder, File } from 'lucide-react';

interface FolderCardProps {
  item: {
    id: string;
    name: string;
    type: string;
    enabled: boolean;
    imageUrl: string | null;
  };
  isSelected: boolean;
  onNavigate: (name: string) => void;
  toggleSelection: (id: string, multi: boolean) => void;
  clearSelection: () => void;
}

export default function FolderCard({
  item,
  isSelected,
  onNavigate,
  toggleSelection,
  clearSelection,
}: FolderCardProps) {
  return (
    <div
      id={`grid-item-${item.id}`}
      onClick={(e) => {
        if (!e.ctrlKey && !e.shiftKey) clearSelection();
        toggleSelection(item.id, e.ctrlKey || e.shiftKey);
      }}
      onDoubleClick={() => item.type === 'folder' && onNavigate(item.name)}
      className={`
        group relative flex flex-col rounded-lg overflow-hidden cursor-pointer transition-all duration-200 border min-h-[180px]
        ${
          isSelected
            ? 'border-primary/50 bg-base-200 shadow-md ring-1 ring-primary/50'
            : 'border-transparent bg-base-200 hover:bg-base-300 hover:shadow-lg hover:-translate-y-1'
        }
      `}
    >
      {/* Thumbnail / Icon Area */}
      <div className="aspect-video bg-base-300 relative flex items-center justify-center overflow-hidden select-none">
        {item.imageUrl ? (
          <img
            src={item.imageUrl}
            alt={item.name}
            className={`w-full h-full object-cover transition-transform duration-500 ${isSelected ? 'scale-105 opacity-100' : 'scale-100 opacity-80 group-hover:scale-105 group-hover:opacity-100'}`}
            draggable={false}
          />
        ) : item.type === 'folder' ? (
          <Folder
            size={48}
            className={`transition-colors duration-300 ${isSelected ? 'text-primary' : 'text-base-content/20 group-hover:text-base-content/40'}`}
          />
        ) : (
          <File
            size={48}
            className="text-base-content/20 group-hover:text-base-content/40 transition-colors"
          />
        )}
      </div>

      {/* Info Area */}
      <div className="p-3">
        <div className="flex items-start justify-between gap-2">
          <h3
            className={`font-medium text-sm truncate leading-tight select-none transition-colors ${isSelected ? 'text-primary' : 'text-base-content/80 group-hover:text-base-content'}`}
            title={item.name}
          >
            {item.name}
          </h3>
        </div>

        <div className="mt-2 flex items-center justify-between">
          <div className="flex items-center gap-1.5">
            <div
              className={`w-2 h-2 rounded-full ${item.enabled ? 'bg-secondary' : 'bg-base-content/20'}`}
            />
            <span className="text-[10px] uppercase font-bold text-base-content/40">
              {item.enabled ? 'Active' : 'Disabled'}
            </span>
          </div>
          {item.type === 'folder' && (
            <span className="text-[9px] text-base-content/20 font-bold tracking-wider">DIR</span>
          )}
        </div>
      </div>
    </div>
  );
}
