import { commands, sparse, type GameObject } from '../../../shared/api/tauri/bindings';
import type {
  ObjectFilter,
  ObjectSummary,
  CategoryCount,
  CreateObjectInput,
  UpdateObjectInput,
} from '@/entities/game-object/model/object';
import { notifyCommittedMutationSyncWarning } from '../../../shared/lib/committedMutationWarning';

/**
 * `get_object` returns `Option<GameObject>`. Re-reading a row we just wrote can
 * only come back empty if it was deleted concurrently, so surface that instead
 * of handing callers a half-typed object.
 */
async function readBack(id: string): Promise<GameObject> {
  const object = await commands.getObject(id);
  if (!object) {
    throw new Error(`Object ${id} disappeared immediately after being written.`);
  }
  return object;
}

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
  const res = await commands.getObjectsCmd(filter);
  return res.objects;
}

export function getCategoryCounts(gameId: string): Promise<CategoryCount[]> {
  return commands.getCategoryCountsCmd(gameId);
}

export async function createObject(input: CreateObjectInput): Promise<GameObject> {
  const nameError = validateObjectName(input.name);
  if (nameError) throw new Error(nameError);

  const result = await commands.createObjectCmd(input);
  notifyCommittedMutationSyncWarning(result);
  // Re-fetch since create_object_cmd only returns ID
  return readBack(result.id);
}

export async function updateObject(id: string, updates: UpdateObjectInput): Promise<GameObject> {
  if (updates.name !== undefined && updates.name !== null) {
    const nameError = validateObjectName(updates.name);
    if (nameError) throw new Error(nameError);
  }

  await commands.updateObjectCmd(id, sparse(updates));
  return readBack(id);
}

export async function deleteObject(id: string, force: boolean): Promise<void> {
  const result = await commands.deleteObjectCmd(id, force);
  notifyCommittedMutationSyncWarning(result);
}
