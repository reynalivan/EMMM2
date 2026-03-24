/**
 * useObjectListHandlers — Orchestrator that composes all sub-hooks.
 *
 * This file acts as the primary connector between individual feature hooks
 * (CRUD, Scan, Archive, Drop, Bulk) and the main ObjectList UI.
 */

import React from 'react';
import type { ModFolder } from '../../hooks/useFolders';
import type { ObjectSummary, GameSchema } from '../../types/object';
import { useObjHandlersCrud } from './useObjHandlersCrud';
import { useObjHandlersScan } from './useObjHandlersScan';
import { useObjHandlersArchive } from './useObjHandlersArchive';
import { useObjHandlersDrop } from './useObjHandlersDrop';
import { useObjHandlersBulk } from './useObjHandlersBulk';

interface HandlerDeps {
  objects: ObjectSummary[];
  folders?: ModFolder[];
  schema: GameSchema | undefined;
  mismatchConfirm: string[] | null;
  setMismatchConfirm: React.Dispatch<React.SetStateAction<string[] | null>>;
  bulkSelect: {
    selectedIds: Set<string>;
    clearSelection: () => void;
  };
}

export function useObjectListHandlers({
  objects,
  folders = [],
  schema,
  mismatchConfirm,
  setMismatchConfirm,
}: HandlerDeps) {
  // ── 1. Feature Hooks ───────────────────────────────────────────
  const crud = useObjHandlersCrud({ objects, folders, schema });
  const scan = useObjHandlersScan({ objects, folders });

  // Archive depends on scan review state to resume flows
  const archive = useObjHandlersArchive({
    objects,
    setScanReview: scan.setScanReview,
    setIsSyncing: scan.setIsSyncing,
    setMismatchConfirm: (paths) => setMismatchConfirm(paths),
  });

  const drop = useObjHandlersDrop({
    objects,
    handleArchivesInteractively: archive.handleArchivesInteractively,
    setMismatchConfirm: (paths) => setMismatchConfirm(paths),
    setScanReview: scan.setScanReview,
    setIsSyncing: scan.setIsSyncing,
  });

  const bulk = useObjHandlersBulk({
    objects,
    toggleObjectMods: crud.toggleObjectMods,
  });

  // ── 2. Mapping to Unified Interface ────────────────────────────
  return {
    // Dialog & Modal States
    deleteDialog: crud.deleteDialog,
    setDeleteDialog: crud.setDeleteDialog,
    editObject: crud.editObject,
    setEditObject: crud.setEditObject,
    deleteObjectDialog: crud.deleteObjectDialog,
    setDeleteObjectDialog: crud.setDeleteObjectDialog,
    forceDeleteObjectDialog: crud.forceDeleteObjectDialog,
    setForceDeleteObjectDialog: crud.setForceDeleteObjectDialog,
    bulkTagModal: bulk.bulkTagModal,
    setBulkTagModal: bulk.setBulkTagModal,
    mismatchConfirm,
    setMismatchConfirm,
    isSyncing: scan.isSyncing,
    syncConfirm: scan.syncConfirm,
    setSyncConfirm: scan.setSyncConfirm,
    scanReview: scan.scanReview,
    handleCommitScan: scan.handleCommitScan,
    handleCloseScanReview: scan.handleCloseScanReview,
    archiveModal: archive.archiveModal,

    // CRUD Handlers
    handleToggle: crud.handleToggle,
    handleOpen: crud.handleOpen,
    handleDelete: crud.handleDelete,
    confirmDelete: crud.confirmDelete,
    handleDeleteObject: crud.handleDeleteObject,
    confirmDeleteObject: crud.confirmDeleteObject,
    confirmForceDeleteObject: crud.confirmForceDeleteObject,
    handleEdit: crud.handleEdit,
    handlePin: crud.handlePin,
    handleFavorite: crud.handleFavorite,
    handleMoveCategory: crud.handleMoveCategory,
    handleToggleObjectMods: crud.toggleObjectMods,
    handleRevealInExplorer: crud.handleRevealInExplorer,
    handleEnableObject: crud.handleEnableObject,
    handleDisableObject: crud.handleDisableObject,
    categoryNames: crud.categoryNames,

    // Scanning & Sync Handlers
    handleSync: scan.handleSync,
    handleBackgroundSync: scan.handleBackgroundSync,
    handleSyncWithDb: scan.handleSyncWithDb,
    handleApplySyncMatch: scan.handleApplySyncMatch,

    // Drop & Ingest Handlers
    handleDropOnItem: drop.handleDropOnItem,
    handleDropAutoOrganize: drop.handleDropAutoOrganize,
    handleDropNewObject: drop.handleDropNewObject,
    handleDropOnNewObjectSubmit: drop.handleDropOnNewObjectSubmit,

    // Archive Handlers
    handleArchivesInteractively: archive.handleArchivesInteractively,
    handleArchiveExtractSubmit: archive.handleArchiveExtractSubmit,
    handleArchiveExtractSkip: archive.handleArchiveExtractSkip,
    handleStopExtraction: archive.handleStopExtraction,

    // Bulk Action Handlers
    handleBulkDelete: bulk.handleBulkDelete,
    handleBulkPin: bulk.handleBulkPin,
    handleBulkEnable: bulk.handleBulkEnable,
    handleBulkDisable: bulk.handleBulkDisable,
    handleBulkAddTags: bulk.handleBulkAddTags,
    handleBulkRemoveTags: bulk.handleBulkRemoveTags,
    handleBulkAutoOrganize: bulk.handleBulkAutoOrganize,
    handleBulkFavorite: bulk.handleBulkFavorite,
    handleBulkSafe: bulk.handleBulkSafe,
  };
}
