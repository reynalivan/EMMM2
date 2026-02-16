export interface Collection {
  id: string;
  name: string;
  game_id: string;
  is_safe_context: boolean;
}

export interface CollectionDetails {
  collection: Collection;
  mod_ids: string[];
}

export interface CreateCollectionInput {
  name: string;
  game_id: string;
  is_safe_context: boolean;
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

export interface UndoCollectionResult {
  restored_count: number;
}

export interface ExportCollectionItem {
  mod_id: string;
  actual_name: string;
  folder_path: string;
}

export interface ExportCollectionPayload {
  version: number;
  name: string;
  game_id: string;
  is_safe_context: boolean;
  items: ExportCollectionItem[];
}

export interface ImportCollectionResult {
  collection_id: string;
  imported_count: number;
  missing: string[];
}
