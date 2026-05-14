import type { ArchiveInfo } from '../../../types/scanner';

export interface ExtractOptions {
  autoRename: boolean;
  disableByDefault: boolean;
  folderNames: Record<string, string>;
  unpackNested: boolean;
}

export interface ArchiveModalProps {
  archives: ArchiveInfo[];
  isOpen: boolean;
  onExtract: (
    selectedPaths: string[],
    passwords: Record<string, string>,
    options?: ExtractOptions,
  ) => Promise<void>;
  onSkip: () => void;
  isExtracting: boolean;
  error?: string | null;
  passwordError?: { path: string; message: string } | null;
  extractProgress?: { current: number; total: number } | null;
  fileProgress?: { fileName: string; fileIndex: number; totalFiles: number } | null;
  onStop: () => void;
  existingFolders?: string[];
  targetObjectName?: string;
}

export interface ArchiveGroups {
  encrypted: ArchiveInfo[];
  unencrypted: ArchiveInfo[];
}

export interface FolderNameValidationMessages {
  empty: string;
  illegal: string;
  reserved: string;
}
