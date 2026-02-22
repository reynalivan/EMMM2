declare module '@tauri-apps/plugin-fs' {
  export function readFile(path: string): Promise<Uint8Array>;
  export function mkdir(path: string, options?: { recursive?: boolean }): Promise<void>;
  export function exists(path: string): Promise<boolean>;
  export function remove(path: string, options?: { recursive?: boolean }): Promise<void>;
}
