import { CheckCircle, Info, AlertTriangle } from 'lucide-react';
import type { ScanPreviewItem } from '../../../types/scanner';

export type { MasterDbEntry } from '../../../types/scanner';
import type { MasterDbEntry } from '../../../types/scanner';

export type ConfidenceTier = 'Manual' | 'Excellent' | 'High' | 'Medium' | 'Low' | 'None';

const CONFIDENCE_TIERS: ConfidenceTier[] = ['Manual', 'Excellent', 'High', 'Medium', 'Low', 'None'];

export function groupByConfidence(
  previewItems: ScanPreviewItem[],
  overrides: Record<string, MasterDbEntry | null>,
): Array<{ tier: ConfidenceTier; items: ScanPreviewItem[] }> {
  const groups = new Map<ConfidenceTier, ScanPreviewItem[]>();
  for (const previewItem of previewItems) {
    const tier = overrides[previewItem.folderPath]
      ? 'Manual'
      : normalizeConfidence(previewItem.confidence);
    const tierItems = groups.get(tier) ?? [];
    tierItems.push(previewItem);
    groups.set(tier, tierItems);
  }
  return CONFIDENCE_TIERS.flatMap((tier) => {
    const tierItems = groups.get(tier);
    return tierItems ? [{ tier, items: tierItems }] : [];
  });
}

function normalizeConfidence(confidence: string): ConfidenceTier {
  return CONFIDENCE_TIERS.includes(confidence as ConfidenceTier)
    ? (confidence as ConfidenceTier)
    : 'None';
}

/** Confidence color and icon mapping. */
export function getConfidenceColor(confidence: string) {
  switch (confidence) {
    case 'Excellent':
      return 'text-success border-success/30 bg-success/5';
    case 'High':
      return 'text-info border-info/30 bg-info/5';
    case 'Medium':
      return 'text-warning border-warning/30 bg-warning/5';
    case 'Low':
      return 'text-error border-error/30 bg-error/5';
    default:
      return 'text-base-content/50 border-base-content/20';
  }
}

export function getConfidenceIcon(confidence: string) {
  switch (confidence) {
    case 'Excellent':
    case 'High':
      return <CheckCircle size={10} />;
    case 'Medium':
      return <Info size={10} />;
    case 'Low':
      return <AlertTriangle size={10} />;
    default:
      return null;
  }
}
