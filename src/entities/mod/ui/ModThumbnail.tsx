import { ImageOff } from 'lucide-react';
import { useState } from 'react';
import { useThumbnail } from '../api/useThumbnail';

interface ModThumbnailProps {
  gameId: string;
  folderPath?: string | null;
  thumbnailSrc?: string | null;
  sizeClassName: string;
}

export function ModThumbnail({
  gameId,
  folderPath,
  thumbnailSrc,
  sizeClassName,
}: ModThumbnailProps) {
  const [imageFailed, setImageFailed] = useState(false);
  const shouldResolve = !thumbnailSrc && !!folderPath;
  const thumbnail = useThumbnail(gameId, folderPath ?? '', shouldResolve);
  const src = thumbnailSrc ?? thumbnail.data;
  const isLoading = shouldResolve && thumbnail.isLoading;

  return (
    <div
      className={`${sizeClassName} shrink-0 overflow-hidden rounded-md bg-base-300/70 flex items-center justify-center`}
      aria-hidden="true"
    >
      {isLoading ? (
        <span className="skeleton h-full w-full bg-base-300" />
      ) : src && !imageFailed ? (
        <img
          src={src}
          alt=""
          decoding="async"
          className="h-full w-full object-cover"
          onError={() => setImageFailed(true)}
        />
      ) : (
        <ImageOff className="h-1/2 w-1/2 text-base-content/25" />
      )}
    </div>
  );
}
