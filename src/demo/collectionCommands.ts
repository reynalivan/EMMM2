import type {
  ApplyPreview,
  ApplyResult,
  CollectionPreview,
  CollectionRuntimeSnapshot,
  CollectionRuntimeDescriptor,
  PreviewTreeNode,
  ProjectedCollectionState,
} from '@/shared/api/tauri/bindings.gen';
import { demoCollections } from './collectionData';
import { getDemoCollectionPreview } from './collectionPreviewData';
import { handled, type DemoCommandResult } from './commandResult';

let activeId = 'demo-collection-story';
let partial = false;
let liveTree: PreviewTreeNode[] | null = null;

function preview(id: string): CollectionPreview {
  const collection = demoCollections.find((item) => item.id === id);
  if (!collection) throw new Error(`Unknown demo collection: ${id}`);
  const tree = getDemoCollectionPreview(id).tree_nodes.map((object) => {
    const disabled = id === 'demo-collection-photo' && object.id === 'demo-object-interface';
    const children = disabled
      ? []
      : object.children.flatMap((child) => {
          if (child.is_effectively_active) return [{ ...child, node_type: 'FlatModRoot' }];
          if (id !== 'demo-collection-photo') return [];
          return [{ ...child, node_type: 'FlatModRoot', is_enabled: true, status_kind: 'missing' }];
        });
    return {
      ...object,
      is_enabled: !disabled,
      is_effectively_active: !disabled,
      children,
      mod_count: children.length,
    };
  });
  const state = projected(tree);
  return {
    collection: {
      ...collection,
      is_active: activeId === id && !partial,
      mod_count: state.summary.active_root_count,
    },
    tree_nodes: tree,
    projected_state: state,
  };
}

function projected(tree: PreviewTreeNode[]): ProjectedCollectionState {
  const roots = tree.flatMap((object) =>
    object.children.map((node) => ({
      object_id: object.id,
      root_key: node.id,
      display_name: node.name,
      root_type: 'FlatModRoot',
      source_path: node.path ?? node.name,
      thumbnail_hint: node.path,
      warnings: node.warnings,
      is_missing: node.status_kind === 'missing',
      is_safe: true,
      safety_source: 'manual',
    })),
  );
  return {
    object_states: tree.map((object) => ({
      object_id: object.id,
      display_name: object.name,
      path_key: object.path ?? object.id,
      is_enabled: object.is_enabled,
      active_root_count: object.children.length,
    })),
    active_roots: roots,
    summary: {
      object_count: tree.length,
      enabled_object_count: tree.filter((node) => node.is_enabled).length,
      active_root_count: roots.length,
      missing_root_count: roots.filter((root) => root.is_missing).length,
    },
  };
}

function runtime(): CollectionRuntimeSnapshot {
  const current = preview(activeId);
  const tree = liveTree ?? current.tree_nodes;
  return {
    game_id: 'demo-zenless',
    active_collection_id: activeId,
    active_collection_name: current.collection.name,
    current_signature: current.collection.signature ?? '',
    is_dirty: partial,
    runtime_status: partial ? 'modified' : 'clean',
    is_safe: true,
    is_safety_classified: true,
    missing_count: current.projected_state.summary.missing_root_count,
    last_changes: null,
    current_mods: [],
    current_objects: [],
    current_tree_nodes: tree,
    projected_state: projected(tree),
  };
}

function descriptor(): CollectionRuntimeDescriptor {
  const state = runtime();
  return {
    game_id: state.game_id,
    active_collection_id: state.active_collection_id,
    active_collection_name: state.active_collection_name,
    runtime_status: state.runtime_status,
    missing_count: state.missing_count,
    last_changes: null,
    safety: { is_safe: true, is_safety_classified: true },
    counts: {
      active_mod_count: state.projected_state.summary.active_root_count,
      object_count: state.projected_state.summary.object_count,
      enabled_object_count: state.projected_state.summary.enabled_object_count,
    },
  };
}

function applyPreview(id: string): ApplyPreview {
  const target = preview(id);
  const current = runtime();
  return {
    collection_name: target.collection.name,
    current_state_name: current.active_collection_name,
    current_state_is_unsaved: current.is_dirty,
    current_tree_nodes: current.current_tree_nodes,
    target_tree_nodes: target.tree_nodes,
    effective_target_tree_nodes: target.tree_nodes,
    current_projected_state: current.projected_state,
    target_projected_state: target.projected_state,
    effective_target_projected_state: target.projected_state,
    safe_mode_enabled: false,
  };
}

function apply(id: string, ignoreMissing: boolean): ApplyResult {
  const target = preview(id);
  const currentKeys = new Set(runtime().projected_state.active_roots.map((root) => root.root_key));
  const missing = target.projected_state.active_roots
    .filter((root) => root.is_missing)
    .map((root) => root.source_path);
  if (missing.length > 0 && !ignoreMissing) {
    throw { type: 'MissingMods', payload: { count: missing.length, paths: missing } };
  }
  activeId = id;
  partial = missing.length > 0;
  liveTree = target.tree_nodes.map((object) => {
    const children = object.children.filter((node) => node.status_kind !== 'missing');
    return { ...object, children, mod_count: children.length };
  });
  const targetKeys = new Set(projected(liveTree).active_roots.map((root) => root.root_key));
  const enabled = [...targetKeys].filter((key) => !currentKeys.has(key)).length;
  const disabled = [...currentKeys].filter((key) => !targetKeys.has(key)).length;
  return {
    mods_enabled: enabled,
    mods_disabled: disabled,
    warnings: [],
    final_state_name: target.collection.name,
    partial_apply: partial,
    skipped_missing_paths: missing,
    runtime_path_rewrites: [],
    sync_warning: null,
  };
}

export function resolveDemoCollectionCommand(
  name: string,
  args: unknown[],
): DemoCommandResult | null {
  switch (name) {
    case 'listCollections':
      return handled(demoCollections.map((collection) => preview(collection.id).collection));
    case 'getCollectionRuntimeState':
      return handled(runtime());
    case 'getCollectionRuntimeDescriptor':
      return handled(descriptor());
    case 'getCollectionPreview':
      return handled(preview(String(args[0] ?? '')));
    case 'previewApplyCollection':
      return handled(applyPreview(String(args[1] ?? '')));
    case 'applyCollection': {
      try {
        return handled(apply(String(args[1] ?? ''), args[2] === true));
      } catch (error) {
        return handled(Promise.reject(error));
      }
    }
    case 'getApplyProgress':
      return handled(null);
    case 'getReloadKey':
      return handled('F10');
    default:
      return null;
  }
}
