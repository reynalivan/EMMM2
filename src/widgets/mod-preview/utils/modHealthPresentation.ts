import type {
  ModHealthControl,
  ModHealthAssetCategory,
  ModHealthAssetEntry,
  ModHealthIssue,
  ModHealthPanelData,
  ModHealthSeverity,
  ModHealthSupportLevel,
} from '../components/ModHealthSection';

interface BackendHealthIssue {
  severity: ModHealthSeverity;
  code: string;
  message: string;
  file_path: string | null;
  section: string | null;
  line: number | null;
}

interface BackendManifestCounts {
  referenced: number;
  inactive_only: number;
  orphan: number;
  external_reference: number;
}

interface BackendAssetEntry {
  relative_path: string;
  size_bytes: number;
  source_files: string[];
}

interface BackendControl {
  kind: 'key_toggle' | 'menu_toggle' | 'present' | 'shape_variable';
  section: string;
  file_path: string;
  key: string | null;
  back: string | null;
  variable: string | null;
  values: string[];
  default_value: string | null;
}

export interface ModHealthReportData {
  support_level: 'basic' | 'supported' | 'experimental';
  issues: BackendHealthIssue[];
  manifest: {
    referenced: BackendAssetEntry[];
    inactive_only: BackendAssetEntry[];
    orphan: BackendAssetEntry[];
    external_reference: BackendAssetEntry[];
    counts: BackendManifestCounts;
  };
  file_manifest: unknown[];
  controls: BackendControl[];
}

function toSupportLevel(level: ModHealthReportData['support_level']): ModHealthSupportLevel {
  if (level === 'supported') return 'full';
  if (level === 'experimental') return 'experimental';
  return 'baseline';
}

function toControl(control: BackendControl): ModHealthControl {
  return {
    kind: control.kind === 'shape_variable' ? 'shape' : control.kind,
    name: control.variable ?? control.section,
    key: control.key,
    back: control.back,
    values: control.values,
    defaultValue: control.default_value,
  };
}

function toIssue(issue: BackendHealthIssue): ModHealthIssue {
  return {
    severity: issue.severity,
    code: issue.code,
    message: issue.message,
    filePath: issue.file_path,
    line: issue.line,
  };
}

function toAssetEntries(report: ModHealthReportData): ModHealthAssetEntry[] {
  const categories: Array<[ModHealthAssetCategory, BackendAssetEntry[]]> = [
    ['referenced', report.manifest.referenced],
    ['inactive_only', report.manifest.inactive_only],
    ['orphan', report.manifest.orphan],
    ['external_reference', report.manifest.external_reference],
  ];

  return categories.flatMap(([category, entries]) =>
    entries.map((entry) => ({
      category,
      relativePath: entry.relative_path,
      sizeBytes: entry.size_bytes,
      sourceFiles: entry.source_files,
    })),
  );
}

export function toModHealthPanelData(report: ModHealthReportData): ModHealthPanelData {
  const { counts } = report.manifest;

  return {
    supportLevel: toSupportLevel(report.support_level),
    issues: report.issues.map(toIssue),
    assets: {
      referenced: counts.referenced,
      inactiveOnly: counts.inactive_only,
      orphan: counts.orphan,
      externalReference: counts.external_reference,
    },
    assetEntries: toAssetEntries(report),
    controls: report.controls.map(toControl),
  };
}
