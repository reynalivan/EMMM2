export interface DemoCommandResult {
  handled: boolean;
  value: unknown;
}

export function handled(value: unknown): DemoCommandResult {
  return { handled: true, value };
}
