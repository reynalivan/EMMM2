import { Lock } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ObjectSummary } from '../../types/object';

export interface ObjectListBannersProps {
  selectedObject: ObjectSummary | null;
  onEnable: (id: string) => void;
}

export function ObjectListBanners({ selectedObject, onEnable }: ObjectListBannersProps) {
  const { t } = useTranslation(['objects']);

  if (!selectedObject || selectedObject.is_enabled) {
    return null;
  }

  return (
    <div className="mx-2 mt-1 mb-0.5 flex items-center gap-2 bg-error/10 border border-error/20 rounded-md px-2 py-1.5 shadow-sm animate-in fade-in slide-in-from-top-1">
      <Lock size={12} className="text-error shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-[10px] font-bold text-error/90 leading-tight truncate">
          {t('banners.object_disabled_title', { name: selectedObject.name })}
        </p>
      </div>
      <button
        className="btn btn-xs btn-error px-3 font-bold shadow-sm"
        onClick={() => onEnable(selectedObject.id)}
      >
        {t('banners.enable_object_btn')}
      </button>
    </div>
  );
}
