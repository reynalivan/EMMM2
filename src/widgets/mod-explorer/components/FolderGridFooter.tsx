import { useTranslation } from 'react-i18next';
import { LiquidSurface } from '@/shared/ui/liquid';

interface FolderGridFooterProps {
  visibleCount: number;
}

export default function FolderGridFooter({ visibleCount }: FolderGridFooterProps) {
  const { t } = useTranslation(['grid']);

  return (
    <footer className="pointer-events-none flex justify-end" data-testid="folder-grid-footer">
      <LiquidSurface
        liquidRole="overlay"
        className="rounded-lg px-2.5 py-1 text-[10px] font-medium tabular-nums text-base-content/55 shadow-sm"
      >
        <span aria-live="polite">{t('toolbar.item_count', { count: visibleCount })}</span>
      </LiquidSurface>
    </footer>
  );
}
