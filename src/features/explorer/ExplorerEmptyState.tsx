import { MousePointerClick } from 'lucide-react';

export default function ExplorerEmptyState() {
  return (
    <div className="flex flex-col items-center justify-center h-full text-base-content/30 p-8 text-center select-none bg-base-100/50">
      <div className="w-24 h-24 bg-base-200/50 rounded-full flex items-center justify-center mb-6 ring-1 ring-base-content/5">
        <MousePointerClick size={40} className="opacity-50" strokeWidth={1.5} />
      </div>
      <h3 className="text-xl font-bold text-base-content/70 mb-2">No Object Selected</h3>
      <p className="max-w-xs text-base-content/50 leading-relaxed">
        Select a category from the sidebar (like <strong>Characters</strong> or{' '}
        <strong>Weapons</strong>) to view and manage your mods.
      </p>
    </div>
  );
}
