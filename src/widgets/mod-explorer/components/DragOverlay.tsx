import { Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface DragOverlayProps {
  isDragging: boolean;
}

export default function DragOverlay({ isDragging }: DragOverlayProps) {
  const { t } = useTranslation('folder_grid');

  if (!isDragging) return null;

  return (
    <div className="absolute inset-0 z-50 bg-primary/10 backdrop-blur-sm border-2 border-primary border-dashed rounded-lg flex flex-col items-center justify-center animate-in fade-in duration-200 pointer-events-none">
      <div className="bg-base-100 p-6 rounded-xl shadow-xl flex flex-col items-center gap-3">
        <Upload size={48} className="text-primary animate-bounce" />
        <div className="text-center">
          <h3 className="font-bold text-lg">{t('drag_overlay.title')}</h3>
          <p className="text-sm opacity-60">{t('drag_overlay.subtitle')}</p>
        </div>
      </div>
    </div>
  );
}
