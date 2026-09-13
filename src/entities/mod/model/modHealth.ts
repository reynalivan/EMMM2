export type ModViewerExternalChangeKind = 'added' | 'modified' | 'removed';

export type ModViewerExternalChangeCategory = 'ini' | 'dds' | 'backup' | 'metadata' | 'other';

export interface ModViewerExternalReview {
  changes: Array<{
    kind: ModViewerExternalChangeKind;
    category: ModViewerExternalChangeCategory;
    relativePath: string;
  }>;
  collectionImpact: string | null;
}
