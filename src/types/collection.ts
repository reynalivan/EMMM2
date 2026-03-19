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
  object_states: CollectionObjectState[];
}

export interface CollectionObjectState {
  object_id: string;
  is_enabled: boolean;
}

export interface CollectionPreviewMod {
  id: string;
  actual_name: string;
  folder_path: string;
  is_safe: boolean;
  object_id: string | null;
  object_name: string | null;
  object_type: string | null;
  node_type?: 'ModPackRoot' | 'FlatModRoot' | 'VariantContainer' | null;
}

export type CollectionStateKind = 'named' | 'unsaved' | 'none';

export interface RuntimeObjectState {
  object_id: string;
  name: string;
  object_type: string;
  is_enabled: boolean;
  thumbnail_hint: string | null;
}

export interface CorridorRuntimeSnapshot {
  game_id: string;
  is_safe: boolean;
  active_collection_id: string | null;
  state_name: string | null;
  state_kind: CollectionStateKind;
  roots: CollectionPreviewMod[];
  object_states: RuntimeObjectState[];
  signature: string;
  snapshot_source: string;
  reconciled_count: number;
}

export interface CollectionRuntimePreview {
  collection: Collection;
  roots: CollectionPreviewMod[];
  object_states: RuntimeObjectState[];
  signature: string;
}

export interface CorridorPreview {
  leaving_mods: CollectionPreviewMod[];
  leaving_object_states: RuntimeObjectState[];
  leaving_state_name: string;
  leaving_state_kind: Exclude<CollectionStateKind, 'none'>;
  target_mods: CollectionPreviewMod[];
  target_object_states: RuntimeObjectState[];
  target_state_name: string | null;
  target_state_kind: CollectionStateKind;
  target_description: string;
}

export type SaveCollectionMode = 'current_state' | 'snapshot_collection';

export interface CreateCollectionInput {
  name: string;
  game_id: string;
  is_safe_context: boolean;
  auto_snapshot?: boolean;
  mod_ids: string[];
  object_states?: CollectionObjectState[];
}

export interface UpdateCollectionInput {
  id: string;
  game_id: string;
  name?: string;
  is_safe_context?: boolean;
  mod_ids?: string[];
  object_states?: CollectionObjectState[];
}

export interface ApplyCollectionResult {
  changed_count: number;
  warnings: string[];
}

export type ApplyCollectionProgressPhase =
  | 'idle'
  | 'preparing'
  | 'renaming'
  | 'updating_db'
  | 'done'
  | 'failed';

export interface ApplyCollectionProgress {
  phase: ApplyCollectionProgressPhase;
  completed: number;
  total: number;
  current_item: string | null;
  collection_name: string | null;
  is_safe: boolean | null;
  error: string | null;
}
