export interface Collection {
  id: string;
  name: string;
  game_id: string;
  is_safe_context: boolean;
  member_count: number;
  is_last_unsaved: boolean;
}

export interface CollectionDetails {
  collection: Collection;
  mod_ids: string[];
}

export interface CollectionPreviewMod {
  id: string;
  actual_name: string;
  folder_path: string;
  is_safe: boolean;
  object_id: string | null;
  object_name: string | null;
  object_type: string | null;
}

export interface CreateCollectionInput {
  name: string;
  game_id: string;
  is_safe_context: boolean;
  auto_snapshot?: boolean;
  mod_ids: string[];
}

export interface UpdateCollectionInput {
  id: string;
  game_id: string;
  name?: string;
  is_safe_context?: boolean;
  mod_ids?: string[];
}

export interface ApplyCollectionResult {
  changed_count: number;
  warnings: string[];
}
