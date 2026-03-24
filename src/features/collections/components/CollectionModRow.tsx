import { useEffect, useRef, useState } from 'react';

import { commands } from '../../../lib/bindings';
import { useAppStore } from '../../../stores/useAppStore';
import { Folder, FolderTree } from 'lucide-react';
import type { CollectionMember } from '../../../types/collection';

interface CollectionModRowProps {
  mod: CollectionMember;
}

export function CollectionModRow({ mod }: CollectionModRowProps) {
  const activeGameId = useAppStore((state) => state.activeGameId);
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
  }, [mod.path_key]);

  useEffect(() => {
    if (!isVisible || !mod.path_key || !activeGameId) {
      return;
    }
    setThumbnailUrl(null);
    commands
      .getModThumbnail({ gameId: activeGameId, folderPath: mod.path_key })
      .then((path: string | null) => {
        if (path) {
          setThumbnailUrl(path);
        }
      })
      .catch((error: unknown) => {
        console.warn('Failed to load mod thumbnail', error);
      });
  }, [isVisible, mod.path_key, activeGameId]);

  return (
    <div
      ref={rowRef}
      className="flex items-center gap-3 px-3 py-1.5 hover:bg-base-300/30 transition-colors rounded-md group"
      title={mod.path_key ?? undefined}
    >
      <div className="w-7 h-7 rounded-md bg-base-300 overflow-hidden shrink-0 flex items-center justify-center border border-base-content/5">
        {thumbnailUrl ? (
          <img src={thumbnailUrl} alt="" className="w-full h-full object-cover" />
        ) : (
          <Folder size={12} className="text-base-content/20" />
        )}
      </div>

      <span className="text-xs font-medium truncate flex-1 text-base-content/80 group-hover:text-base-content transition-colors">
        {mod.display_name || (mod.path_key ? mod.path_key.split('/').pop() : 'Unknown Mod')}
      </span>

      <div className="flex items-center gap-1 shrink-0">
        {mod.kind === 'nested' && (
          <span title="Nested mod" className="flex shrink-0">
            <FolderTree size={11} className="text-info/60" />
          </span>
        )}
      </div>
    </div>
  );
}
