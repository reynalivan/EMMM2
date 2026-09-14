import { Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface DragOverlayProps {
  isDragging: boolean;
}

export default function DragOverlay({ isDragging }: DragOverlayProps) {
  const { t } = useTranslation('folder_grid');

  if (!isDragging) return null;

  return (
    <div className="workspace-transient-enter pointer-events-none absolute inset-0 z-50 flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-primary bg-primary/10 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-3 rounded-xl border border-base-300 bg-base-100 p-6">
        <Upload size={48} className="text-primary" />
        <div className="text-center">
          <h3 className="font-bold text-lg">{t('drag_overlay.title')}</h3>
          <p className="text-sm opacity-60">{t('drag_overlay.subtitle')}</p>
        </div>
      </div>
    </div>
  );
}
