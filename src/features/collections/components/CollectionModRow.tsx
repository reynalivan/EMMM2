import { useEffect, useRef, useState } from 'react';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { Folder, FolderTree, ShieldAlert } from 'lucide-react';
import { getPreviewModDisplayName } from '../utils/previewModDisplayName';
import type { CollectionPreviewMod } from '../../../types/collection';

interface CollectionModRowProps {
  mod: CollectionPreviewMod;
}

export function CollectionModRow({ mod }: CollectionModRowProps) {
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [isVisible, setIsVisible] = useState<boolean>(false);
  const rowRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const element = rowRef.current;
    if (!element) {
      return;
    }
    if (typeof IntersectionObserver === 'undefined') {
      setIsVisible(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setIsVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: '120px' },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [mod.folder_path]);

  useEffect(() => {
    if (!isVisible) {
      return;
    }
    setThumbnailUrl(null);
    invoke<string | null>('get_mod_thumbnail', { folderPath: mod.folder_path })
      .then((path) => {
        if (path) {
          setThumbnailUrl(convertFileSrc(path));
        }
      })
      .catch((error: unknown) => {
        console.warn('Failed to load mod thumbnail', error);
      });
  }, [isVisible, mod.folder_path]);

  return (
    <div
      ref={rowRef}
      className="flex items-center gap-3 px-3 py-1.5 hover:bg-base-300/30 transition-colors rounded-md group"
      title={mod.folder_path}
    >
      <div className="w-7 h-7 rounded-md bg-base-300 overflow-hidden shrink-0 flex items-center justify-center border border-base-content/5">
        {thumbnailUrl ? (
          <img src={thumbnailUrl} alt="" className="w-full h-full object-cover" />
        ) : (
          <Folder size={12} className="text-base-content/20" />
        )}
      </div>

      <span className="text-xs font-medium truncate flex-1 text-base-content/80 group-hover:text-base-content transition-colors">
        {getPreviewModDisplayName(mod)}
      </span>

      <div className="flex items-center gap-1 shrink-0">
        {mod.id.startsWith('nested_') && (
          <span title="Nested mod" className="flex shrink-0">
            <FolderTree size={11} className="text-info/60" />
          </span>
        )}
        {!mod.is_safe && (
          <span title="Unsafe" className="flex shrink-0">
            <ShieldAlert size={11} className="text-error/60" />
          </span>
        )}
      </div>
    </div>
  );
}
