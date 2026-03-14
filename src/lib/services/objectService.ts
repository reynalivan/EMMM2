import { invoke } from '@tauri-apps/api/core';
import type {
  ObjectSummary,
  ObjectFilter,
  CategoryCount,
  UpdateObjectInput,
  CreateObjectInput,
  GetObjectsResult,
} from '../../types/object';
import { useToastStore } from '../../stores/useToastStore';

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

export async function getObjects(filter: ObjectFilter): Promise<ObjectSummary[]> {
  const result = await invoke<GetObjectsResult>('get_objects_cmd', { filter });

  if (result.lost_objects && result.lost_objects.length > 0) {
    // Show a consolidated toast message indicating what was lost
    const count = result.lost_objects.length;
    let message = `Lost ${count} object${count > 1 ? 's' : ''} (directory missing on disk). Local DB synchronized.`;
    if (count <= 3) {
      message += `\n(${result.lost_objects.join(', ')})`;
    }

    useToastStore.getState().addToast('warning', message, 5000);
  }

  return result.objects;
}

export function getCategoryCounts(
  gameId: string,
  safeMode: boolean = false,
): Promise<CategoryCount[]> {
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

/** Garbage-collect objects whose disk folders are missing. Called at sync points only. */
export function gcLostObjects(gameId: string): Promise<string[]> {
  return invoke<string[]>('gc_lost_objects_cmd', { gameId });
}
