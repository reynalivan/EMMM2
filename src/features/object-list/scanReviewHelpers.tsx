import { CheckCircle, Info, AlertTriangle } from 'lucide-react';

/** MasterDB entry for the override search dropdown. */
export interface MasterDbEntry {
  name: string;
  object_type: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
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

/** Map staged match level to user-friendly label. */
export function matchLevelLabel(level: string): string {
  switch (level) {
    case 'AutoMatched':
      return 'Auto-match';
    case 'NeedsReview':
      return 'Review';
    case 'NoMatch':
      return 'No Match';
    default:
      return level;
  }
}
