import type { UpdateObjectInput } from '../../types/object';

export function createObjectUpdate(patch: Partial<UpdateObjectInput>): UpdateObjectInput {
  return {
    name: null,
    object_type: null,
    sub_category: null,
    status: null,
    metadata: null,
    hash_db: null,
    custom_skins: null,
    thumbnail_path: null,
    is_auto_sync: null,
    tags: null,
    ...patch,
  };
}
