/**
 * useObjectListHandlers — orchestrator that composes the ObjectList domains.
 *
 * Three domains: object CRUD (shared with the rest of the workspace), scan
 * review, and import (drop → optional archive extraction → review).
 */

import type { GameSchema } from '@/entities/game-object';
import type { WorkspaceObjectNode } from '@/entities/workspace';
import { useSharedObjectActions } from '@/features/workspace-runtime';
import { useScanReviewFlow } from './useScanReviewFlow';
import { useDropImportFlow } from './useDropImportFlow';
import { useObjectBulkActions } from './useObjectBulkActions';

interface HandlerDeps {
  objects: WorkspaceObjectNode[];
  schema: GameSchema | undefined;
}

export function useObjectListHandlers({ objects, schema }: HandlerDeps) {
  // ── 1. Feature Hooks ───────────────────────────────────────────
  const crud = useSharedObjectActions({ objects, schema });
  const scan = useScanReviewFlow(objects.map((object) => object.id));

  const drop = useDropImportFlow({
    objects,
  });

  const bulk = useObjectBulkActions({
    objects,
  });

  // ── 2. Mapping to Unified Interface ────────────────────────────
  return {
    // Dialog & Modal States
    editObject: crud.editObject,
    setEditObject: crud.setEditObject,
    deleteObjectDialog: crud.deleteObjectDialog,
    setDeleteObjectDialog: crud.setDeleteObjectDialog,
    forceDeleteObjectDialog: crud.forceDeleteObjectDialog,
    setForceDeleteObjectDialog: crud.setForceDeleteObjectDialog,
    bulkTagModal: bulk.bulkTagModal,
    setBulkTagModal: bulk.setBulkTagModal,
    isSyncing: scan.isSyncing,
    isObjectBulkSwitchPending: bulk.isBulkSwitchPending,

    // CRUD Handlers
    handleDeleteObject: crud.handleDeleteObject,
    confirmDeleteObject: crud.confirmDeleteObject,
    confirmForceDeleteObject: crud.confirmForceDeleteObject,
    handleEdit: crud.handleEdit,
    handlePin: crud.handlePin,
    handleMoveCategory: crud.handleMoveCategory,
    handleRevealInExplorer: crud.handleRevealInExplorer,
    handleEnableObject: crud.handleEnableObject,
    handleDisableObject: crud.handleDisableObject,
    isSwitchPending: crud.isSwitchPending,
    isObjectSwitchPending: crud.isObjectSwitchPending,
    categoryNames: crud.categoryNames,

    // Scanning & Sync Handlers
    handleSync: scan.handleSync,
    handleBackgroundSync: scan.handleBackgroundSync,
    handleSyncWithDb: crud.handleSyncWithDb,

    // Drop & Ingest Handlers
    handleDropOnItem: drop.handleDropOnItem,
    handleDropAutoOrganize: drop.handleDropAutoOrganize,
    handleDropOnNewObjectSubmit: drop.handleDropOnNewObjectSubmit,

    // Archive Handlers

    // Bulk Action Handlers
    handleBulkDelete: bulk.handleBulkDelete,
    handleBulkPin: bulk.handleBulkPin,
    handleBulkEnable: bulk.handleBulkEnable,
    handleBulkDisable: bulk.handleBulkDisable,
    handleBulkAddTags: bulk.handleBulkAddTags,
    handleBulkRemoveTags: bulk.handleBulkRemoveTags,
    handleBulkClassifyAndMatch: bulk.handleBulkClassifyAndMatch,
    handleBulkFavorite: bulk.handleBulkFavorite,
    handleBulkSafe: bulk.handleBulkSafe,
  };
}
