declare module '@tauri-apps/plugin-fs' {
  export function readFile(path: string): Promise<Uint8Array>;
  // Add other methods if needed
}
