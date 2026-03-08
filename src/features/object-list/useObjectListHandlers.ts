/**
 * useObjectListHandlers — Orchestrator that composes all sub-hooks.
 *
 * Delegates to:
 *   - useObjHandlersCrud   (CRUD, pin/fav, object toggle)
 *   - useObjHandlersScan   (scan preview, commit, sync with DB)
 *   - useObjHandlersArchive (archive modal, extraction)
 *   - useObjHandlersDrop   (DnD zones)
 *   - useObjHandlersBulk   (bulk operations)
 *
 * Public API is unchanged — consumers see the same return shape.
 */

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
}

export function useObjectListHandlers({
  objects,
  folders = [],
  schema,
  setMismatchConfirm,
}: HandlerDeps) {
  // ── CRUD, misc, and object toggle ──────────────────────────────
  const crud = useObjHandlersCrud({ objects, folders, schema });

  // ── Scan & sync ────────────────────────────────────────────────
  const scan = useObjHandlersScan({ objects, folders });

  // ── Archive modal & extraction ─────────────────────────────────
  const archive = useObjHandlersArchive({
    objects,
    setScanReview: scan.setScanReview,
    setIsSyncing: scan.setIsSyncing,
    setMismatchConfirm,
  });

  // ── DnD drop zones ─────────────────────────────────────────────
  const drop = useObjHandlersDrop({
    objects,
    handleArchivesInteractively: archive.handleArchivesInteractively,
    setScanReview: scan.setScanReview,
    setIsSyncing: scan.setIsSyncing,
    setMismatchConfirm,
  });

  // ── Bulk operations ────────────────────────────────────────────
  const bulk = useObjHandlersBulk({
    objects,
    toggleObjectMods: crud.toggleObjectMods,
  });

  return {
    // Dialog state
    deleteDialog: crud.deleteDialog,
    setDeleteDialog: crud.setDeleteDialog,
    editObject: crud.editObject,
    setEditObject: crud.setEditObject,
    isSyncing: scan.isSyncing,
    syncConfirm: scan.syncConfirm,
    setSyncConfirm: scan.setSyncConfirm,
    scanReview: scan.scanReview,
    handleCommitScan: scan.handleCommitScan,
    handleCloseScanReview: scan.handleCloseScanReview,

    // Handlers
    handleToggle: crud.handleToggle,
    handleOpen: crud.handleOpen,
    handleDelete: crud.handleDelete,
    confirmDelete: crud.confirmDelete,
    handleDeleteObject: crud.handleDeleteObject,
    deleteObjectDialog: crud.deleteObjectDialog,
    setDeleteObjectDialog: crud.setDeleteObjectDialog,
    confirmDeleteObject: crud.confirmDeleteObject,
    handleEdit: crud.handleEdit,
    handleSync: scan.handleSync,
    handleSyncWithDb: scan.handleSyncWithDb,
    handleApplySyncMatch: scan.handleApplySyncMatch,
    handlePin: crud.handlePin,
    handleFavorite: crud.handleFavorite,
    handleMoveCategory: crud.handleMoveCategory,
    handleRevealInExplorer: crud.handleRevealInExplorer,
    handleEnableObject: crud.handleEnableObject,
    handleDisableObject: crud.handleDisableObject,
    categoryNames: crud.categoryNames,
    handleDropOnItem: drop.handleDropOnItem,
    handleDropAutoOrganize: drop.handleDropAutoOrganize,
    handleDropNewObject: drop.handleDropNewObject,
    handleDropOnNewObjectSubmit: drop.handleDropOnNewObjectSubmit,
    archiveModal: archive.archiveModal,
    handleArchivesInteractively: archive.handleArchivesInteractively,
    handleArchiveExtractSubmit: archive.handleArchiveExtractSubmit,
    handleArchiveExtractSkip: archive.handleArchiveExtractSkip,
    handleStopExtraction: archive.handleStopExtraction,

    // Bulk action handlers
    bulkTagModal: bulk.bulkTagModal,
    setBulkTagModal: bulk.setBulkTagModal,
    handleBulkDelete: bulk.handleBulkDelete,
    handleBulkPin: bulk.handleBulkPin,
    handleBulkEnable: bulk.handleBulkEnable,
    handleBulkDisable: bulk.handleBulkDisable,
    handleBulkAddTags: bulk.handleBulkAddTags,
    handleBulkRemoveTags: bulk.handleBulkRemoveTags,
    handleBulkAutoOrganize: bulk.handleBulkAutoOrganize,
  };
}
