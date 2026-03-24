declare module '@tauri-apps/plugin-fs' {
  export function readFile(path: string): Promise<Uint8Array>;
  export function writeFile(
    path: string,
    data: Uint8Array | number[] | ArrayBuffer,
    options?: Record<string, unknown>,
  ): Promise<void>;
  export function readTextFile(path: string): Promise<string>;
  export function writeTextFile(
    path: string,
    content: string,
    options?: Record<string, unknown>,
  ): Promise<void>;
  export function mkdir(path: string, options?: { recursive?: boolean }): Promise<void>;
  export function exists(path: string): Promise<boolean>;
  export function remove(path: string, options?: { recursive?: boolean }): Promise<void>;
  export function rename(oldPath: string, newPath: string): Promise<void>;
}
