import { Info, Trash2, Copy, Maximize2, X, ChevronRight, Plus } from 'lucide-react';
import { useAppStore } from '../../stores/useAppStore';

export default function PreviewPanel() {
  const { togglePreview, setMobilePane } = useAppStore();
  // TODO: Get selected item details from store
  const selectedName = 'Albedo Flowery';
  const isEnabled = false;

  return (
    <div className="flex flex-col h-full bg-base-100/30 border-l border-white/5 p-6 overflow-y-auto backdrop-blur-md">
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <div className="flex items-center gap-2 mb-1">
            {/* Mobile Back Button */}
            <button
              onClick={() => setMobilePane('grid')}
              className="btn btn-ghost btn-xs btn-circle md:hidden text-white/50 hover:text-white"
            >
              <ChevronRight className="rotate-180" size={16} />
            </button>

            <h2 className="text-xl font-bold text-white tracking-tight glow-text">
              {selectedName}
            </h2>
          </div>

          <label className="label cursor-pointer justify-start gap-2 p-0 hover:opacity-100 opacity-70 transition-opacity">
            <input
              type="checkbox"
              className="toggle toggle-sm border-white/10 bg-base-300 hover:bg-base-200 checked:bg-primary checked:border-primary checked:shadow-[0_0_10px_var(--color-primary)]"
              checked={isEnabled}
              readOnly
            />
            <span className="text-sm font-medium text-white/60">
              {isEnabled ? 'Enabled' : 'Disabled'}
            </span>
          </label>
        </div>

        <div className="flex items-center gap-1">
          <button className="btn btn-ghost btn-sm btn-circle text-error/50 hover:text-error hover:bg-error/10 transition-colors">
            <Trash2 size={18} />
          </button>

          {/* Desktop Close/Toggle Button */}
          <button
            onClick={togglePreview}
            className="btn btn-ghost btn-sm btn-circle text-white/30 hover:text-white hidden md:inline-flex hover:bg-white/5"
            title="Close Preview"
          >
            <ChevronRight size={18} />
          </button>

          {/* Mobile Close Button */}
          <button
            onClick={() => setMobilePane('grid')}
            className="btn btn-ghost btn-sm btn-circle text-white/30 md:hidden hover:text-white"
          >
            <X size={18} />
          </button>
        </div>
      </div>

      {/* Preview Carousel */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-xs font-bold text-white/40 uppercase tracking-widest">
            Preview Images
          </h3>
        </div>

        <div className="carousel w-full rounded-xl border border-white/5 bg-black/40 aspect-3/4 relative group shadow-inner">
          {/* Placeholder Carousel Items */}
          <div id="item1" className="carousel-item w-full relative">
            <img
              src="https://picsum.photos/300/400"
              className="w-full h-full object-cover opacity-90"
            />
            <div className="absolute inset-0 bg-linear-to-t from-black/80 to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex items-end justify-center pb-4">
              <p className="text-white text-[10px] font-bold tracking-wider backdrop-blur-md bg-white/5 border border-white/10 px-3 py-1 rounded-full uppercase">
                Primary Preview
              </p>
            </div>
          </div>

          {/* Carousel Controls (Stub) */}
          <div className="absolute flex justify-between transform -translate-y-1/2 left-2 right-2 top-1/2 opacity-0 group-hover:opacity-100 transition-opacity">
            <a
              href="#item4"
              className="btn btn-circle btn-xs bg-black/50 border-white/10 hover:bg-primary text-white hover:border-primary"
            >
              ❮
            </a>
            <a
              href="#item2"
              className="btn btn-circle btn-xs bg-black/50 border-white/10 hover:bg-primary text-white hover:border-primary"
            >
              ❯
            </a>
          </div>
        </div>

        {/* Carousel Toolbar */}
        <div className="flex justify-center gap-1 mt-3">
          <span className="text-[10px] font-mono text-white/30 mr-auto pt-1">1 / 1</span>
          <button
            className="btn btn-xs btn-ghost btn-square text-white/30 hover:text-white"
            title="Add Image"
          >
            <Plus size={14} />
          </button>
          <button
            className="btn btn-xs btn-ghost btn-square text-white/30 hover:text-white"
            title="Paste Image"
          >
            <Copy size={14} />
          </button>
          <button
            className="btn btn-xs btn-ghost btn-square text-white/30 hover:text-white"
            title="Delete Image"
          >
            <Trash2 size={14} />
          </button>
          <button
            className="btn btn-xs btn-ghost btn-square text-white/30 hover:text-white"
            title="Fullscreen"
          >
            <Maximize2 size={14} />
          </button>
        </div>
      </div>

      {/* Description */}
      <div className="mb-6">
        <h3 className="text-xs font-bold text-white/40 uppercase tracking-widest mb-2">
          Description
        </h3>
        <textarea
          className="textarea textarea-bordered w-full h-24 text-sm bg-transparent border-white/10 focus:border-primary/50 text-white/80 transition-colors resize-none placeholder:text-white/20 focus:bg-white/5"
          placeholder="No description available."
        ></textarea>
      </div>

      {/* Configuration Files */}
      <div>
        <h3 className="text-xs font-bold text-white/40 uppercase tracking-widest mb-2">
          Mod Configuration
        </h3>
        <div className="space-y-2">
          {[
            { file: 'merged.ini', key: 1 },
            { file: 'KeySwap.ini', key: 2 },
          ].map((item) => (
            <div
              key={item.key}
              className="flex items-center justify-between p-3 bg-base-200/20 rounded-lg border border-white/5 group hover:border-primary/50 hover:bg-base-200/40 transition-all cursor-pointer hover:shadow-[0_0_15px_-5px_rgba(var(--color-primary),0.3)]"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded bg-primary/10 flex items-center justify-center text-primary group-hover:text-white group-hover:bg-primary transition-colors">
                  <span className="text-[10px] font-bold">INI</span>
                </div>
                <div className="text-sm font-medium text-white/80 group-hover:text-white">
                  {item.file}
                </div>
              </div>
              <div className="opacity-0 group-hover:opacity-100 transition-opacity">
                <button className="btn btn-xs btn-ghost text-primary hover:bg-primary/20">
                  Edit
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Floating Action Button (Mobile/Small Screens primarily) */}
      <div className="mt-auto pt-6 text-center">
        <button className="btn btn-outline btn-sm w-full gap-2 opacity-50 hover:opacity-100 border-white/20 text-white hover:bg-white/10 hover:border-white/40 hover:text-white">
          <Info size={16} />
          View File Location
        </button>
      </div>
    </div>
  );
}
