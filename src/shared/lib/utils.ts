import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { convertFileSrc } from '@tauri-apps/api/core';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Converts a local asset path or URI to a browser-loadable URL.
 *
 * - Already-valid remote and local URIs pass through unchanged.
 * - Absolute filesystem paths are converted via `convertFileSrc`.
 * - Relative paths or empty strings return '' to avoid broken `<img>` renders.
 */
export function getFileUrl(path: string | null | undefined): string {
  if (!path) return '';

  // The preview API normally returns absolute filesystem paths, while a
  // desktop bridge or cached entry can already supply a loadable URI. Never
  // send an existing URI through `convertFileSrc`, which would otherwise
  // produce an invalid image source.
  if (/^(?:https?|asset|file|data|blob):/i.test(path)) {
    return path;
  }

  // Absolute filesystem path — convert to an asset:// URL via Tauri.
  // Works for Windows drive, UNC, and Unix paths.
  if (
    path.startsWith('/') ||
    path.startsWith('\\\\') ||
    /^[a-zA-Z]:\\/.test(path) ||
    /^[a-zA-Z]:\//.test(path)
  ) {
    return convertFileSrc(path);
  }

  // Relative path or unknown format — returning '' prevents broken image renders.
  return '';
}
