import { ImageIcon, Upload, Trash2 } from 'lucide-react';

interface EditObjectTabThumbnailProps {
  displayThumbnail: string | null;
  selectedThumbnailPath: string | null;
  thumbnailAction: 'keep' | 'update' | 'delete';
  activeTab: 'manual' | 'auto';
  handleThumbnailClick: () => void;
  handleDeleteThumbnail: () => void;
}

export function EditObjectTabThumbnail({
  displayThumbnail,
  selectedThumbnailPath,
  thumbnailAction,
  activeTab,
  handleThumbnailClick,
  handleDeleteThumbnail,
}: EditObjectTabThumbnailProps) {
  return (
    <div className="flex w-full shrink-0 flex-col items-center gap-2 md:w-32">
      <span className="label-text self-center font-medium">Thumbnail</span>
      <div className="w-32 h-32 rounded-xl bg-base-300 overflow-hidden flex items-center justify-center border border-base-content/10 relative shadow-inner">
        {displayThumbnail ? (
          <img
            src={displayThumbnail}
            alt="Thumbnail"
            className={`w-full h-full object-cover ${selectedThumbnailPath ? 'opacity-50' : ''}`}
          />
        ) : (
          <ImageIcon size={48} className="opacity-20" />
        )}
      </div>
      {activeTab === 'manual' && (
        <div className="flex gap-2 w-full">
          <button
            type="button"
            className="btn btn-sm btn-outline flex-1 gap-2"
            onClick={handleThumbnailClick}
          >
            <Upload size={14} />
            Change
          </button>
          <button
            type="button"
            className="btn btn-sm btn-outline btn-error square px-2"
            onClick={handleDeleteThumbnail}
            disabled={!displayThumbnail}
            title="Delete Thumbnail"
          >
            <Trash2 size={14} />
          </button>
        </div>
      )}
      {thumbnailAction === 'update' && (
        <div className="text-xs opacity-50 truncate max-w-32">Selected</div>
      )}
      {thumbnailAction === 'delete' && (
        <div className="text-xs text-error opacity-70">Will be deleted</div>
      )}
    </div>
  );
}
