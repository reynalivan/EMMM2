export interface SwitchWarning {
  stage: 'disable' | 'restore' | 'unknown';
  code: string;
  message: string;
}

export interface CorridorSwitchResult {
  disabled_count: number;
  restored_count: number;
  partial_success?: boolean;
  warnings: string[];
  typed_warnings?: SwitchWarning[];
}
