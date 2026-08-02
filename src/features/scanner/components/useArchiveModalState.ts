import { useCallback, useEffect, useMemo, useState } from 'react';
import type { ArchiveInfo } from '../../../types/scanner';
import type { ExtractOptions, FolderNameValidationMessages } from './archiveModalTypes';
import {
  buildInitialFolderNames,
  buildInitialSelectedPaths,
  findDuplicateFolderNames,
  groupArchivesByEncryption,
  validateFolderName,
} from './archiveModalUtils';

interface UseArchiveModalStateInput {
  archives: ArchiveInfo[];
  validationMessages: FolderNameValidationMessages;
}

export function useArchiveModalState({
  archives,
  validationMessages,
}: UseArchiveModalStateInput) {
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(() =>
    buildInitialSelectedPaths(archives),
  );
  const [passwords, setPasswords] = useState<Record<string, string>>({});
  const [autoRename, setAutoRename] = useState(true);
  const [disableByDefault, setDisableByDefault] = useState(true);
  const [folderNames, setFolderNames] = useState<Record<string, string>>(() =>
    buildInitialFolderNames(archives),
  );
  const [editingPath, setEditingPath] = useState<string | null>(null);
  const [showStopConfirm, setShowStopConfirm] = useState(false);
  const [unpackNested, setUnpackNested] = useState(true);

  useEffect(() => {
    setSelectedPaths(buildInitialSelectedPaths(archives));
    setFolderNames(buildInitialFolderNames(archives));
    setPasswords({});
    setEditingPath(null);
    setShowStopConfirm(false);
  }, [archives]);

  const groups = useMemo(() => groupArchivesByEncryption(archives), [archives]);
  const hasNestedArchives = useMemo(
    () => archives.some((archive) => archive.contains_nested_archives),
    [archives],
  );
  const duplicateNames = useMemo(
    () => findDuplicateFolderNames(selectedPaths, folderNames),
    [selectedPaths, folderNames],
  );

  const validateArchiveFolderName = useCallback(
    (name: string) => validateFolderName(name, validationMessages),
    [validationMessages],
  );

  const hasValidationErrors = useMemo(() => {
    for (const path of selectedPaths) {
      if (validateArchiveFolderName(folderNames[path] ?? '')) {
        return true;
      }
    }

    return false;
  }, [selectedPaths, folderNames, validateArchiveFolderName]);

  const toggleSelection = useCallback((path: string) => {
    setSelectedPaths((previous) => {
      const next = new Set(previous);
      if (next.has(path)) {
        next.delete(path);
        return next;
      }

      next.add(path);
      return next;
    });
  }, []);

  const setPasswordForPath = useCallback((path: string, password: string) => {
    setPasswords((previous) => ({ ...previous, [path]: password }));
  }, []);

  const setFolderName = useCallback((path: string, name: string) => {
    setFolderNames((previous) => ({ ...previous, [path]: name }));
  }, []);

  const buildExtractOptions = useCallback(
    (): ExtractOptions => ({
      autoRename,
      disableByDefault,
      folderNames,
      unpackNested,
    }),
    [autoRename, disableByDefault, folderNames, unpackNested],
  );

  return {
    selectedPaths,
    selectedCount: selectedPaths.size,
    passwords,
    autoRename,
    setAutoRename,
    disableByDefault,
    setDisableByDefault,
    folderNames,
    editingPath,
    setEditingPath,
    showStopConfirm,
    setShowStopConfirm,
    unpackNested,
    setUnpackNested,
    groups,
    hasNestedArchives,
    duplicateNames,
    hasValidationErrors,
    toggleSelection,
    setPasswordForPath,
    setFolderName,
    validateArchiveFolderName,
    buildExtractOptions,
  };
}
