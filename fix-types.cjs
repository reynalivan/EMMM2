const fs = require('fs');
let code = fs.readFileSync('src/pages/browser/types.ts', 'utf8');

// replace the import
code = code.replace(/import type \{ BrowserDownloadDto, ImportJobDto \} from '\.\.\/\.\.\/core\/tauri\/bindings\.gen';/, "import type { BrowserDownloadDto } from '../../shared/api/tauri/bindings.gen';");

// remove ImportJobStatus type
code = code.replace(/export type ImportJobStatus =[\s\S]*?\| 'canceled';/, '');

// remove ImportJobItem
code = code.replace(/export type ImportJobItem = Omit<ImportJobDto, 'status'> & \{ status: ImportJobStatus \};/, '');

// remove ImportJobUpdateEvent
code = code.replace(/\/\/ Runtime import job update event[\s\S]*?export interface ImportJobUpdateEvent \{[\s\S]*?\}/, '');

fs.writeFileSync('src/pages/browser/types.ts', code);
