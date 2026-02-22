import { invoke } from '@tauri-apps/api/core';
import type {
  ObjectSummary,
  ObjectFilter,
  CategoryCount,
  UpdateObjectInput,
  CreateObjectInput,
} from '../types/object';

export function validateObjectName(name: string): string | null {
  const trimmed = name.trim();

  if (!trimmed || trimmed.length < 2) {
    return 'Name must be at least 2 characters.';
  }

  if (trimmed.length > 255) {
    return 'Name must be at most 255 characters.';
  }

  if (/[<>:"/\\|?*]/.test(trimmed)) {
    return 'Name contains invalid characters: < > : " / \\ | ? *';
  }

  const reserved = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
  if (reserved.test(trimmed)) {
    return 'Name is a reserved system name.';
  }

  if (/^\.+$/.test(trimmed)) {
    return 'Name cannot be only dots.';
  }

  if (trimmed.includes('..')) {
    return 'Name cannot contain path traversal (dot-dot).';
  }

  return null;
}

export function getObjects(filter: ObjectFilter): Promise<ObjectSummary[]> {
  return invoke<ObjectSummary[]>('get_objects_cmd', { filter });
}

export function getCategoryCounts(gameId: string, safeMode: boolean): Promise<CategoryCount[]> {
  return invoke<CategoryCount[]>('get_category_counts_cmd', { gameId, safeMode });
}

export function createObject(input: CreateObjectInput): Promise<string> {
  const nameError = validateObjectName(input.name);
  if (nameError) return Promise.reject(new Error(nameError));

  return invoke<string>('create_object_cmd', { input });
}

export function updateObject(id: string, updates: UpdateObjectInput): Promise<void> {
  if (updates.name !== undefined) {
    const nameError = validateObjectName(updates.name);
    if (nameError) return Promise.reject(new Error(nameError));
  }

  return invoke('update_object_cmd', { id, updates });
}

export function deleteObject(id: string): Promise<void> {
  return invoke('delete_object_cmd', { id });
}
