export interface DemoCommandResult {
  handled: boolean;
  value: unknown;
}

const UNHANDLED: DemoCommandResult = { handled: false, value: undefined };

/** Production binding path: always delegate to the generated Tauri command. */
export function resolveDemoCommand(_name: string, _args: unknown[]): DemoCommandResult {
  return UNHANDLED;
}
