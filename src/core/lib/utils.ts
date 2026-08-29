import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { convertFileSrc } from '@tauri-apps/api/core';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Converts a local asset path or URI to a browser-loadable URL.
 *
 * - Already-valid remote URIs (http://, https://) pass through unchanged.
 * - Absolute filesystem paths are converted via `convertFileSrc`.
 * - Relative paths or empty strings return '' to avoid broken `<img>` renders.
 */
export function getFileUrl(path: string | null | undefined): string {
  if (!path) return '';

  // Already-valid remote URIs pass through unchanged.
  if (/^https?:\/\//.test(path)) {
    return path;
  }

  // Absolute filesystem path — convert to an asset:// URL via Tauri.
  // Works for both Windows ('C:\...') and Unix ('/home/...') paths.
  if (path.startsWith('/') || /^[a-zA-Z]:\\/.test(path) || /^[a-zA-Z]:\//.test(path)) {
    return convertFileSrc(path);
  }

  // Relative path or unknown format — returning '' prevents broken image renders.
  return '';
}
