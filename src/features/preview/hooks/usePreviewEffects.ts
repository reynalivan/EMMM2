import { useEffect } from 'react';

interface UsePreviewEffectsOptions {
  activePath: string | null;
  pasteThumbnailFromClipboard: () => Promise<void>;
}

export function usePreviewEffects({
  activePath,
  pasteThumbnailFromClipboard,
}: UsePreviewEffectsOptions): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const isPaste = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'v';
      if (!isPaste || !activePath) {
        return;
      }

      const target = event.target as HTMLElement | null;
      const editable =
        target?.tagName === 'INPUT' ||
        target?.tagName === 'TEXTAREA' ||
        target?.isContentEditable === true;
      if (editable) {
        return;
      }

      event.preventDefault();
      void pasteThumbnailFromClipboard();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [activePath, pasteThumbnailFromClipboard]);
}
