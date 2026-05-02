type StructuredError = {
  type: string;
  payload?: unknown;
};

type FileInUsePayload = {
  path: string;
  processes: string[];
};

type MissingModsPayload = {
  count: number;
  paths: string[];
};

function normalizeStructuredError(value: unknown): StructuredError | null {
  if (!value) {
    return null;
  }

  if (typeof value === 'string') {
    const trimmed = value.trim();
    if (!trimmed.startsWith('{')) {
      return null;
    }

    try {
      return normalizeStructuredError(JSON.parse(trimmed));
    } catch {
      return null;
    }
  }

  if (value instanceof Error) {
    const fromMessage = normalizeStructuredError(value.message);
    if (fromMessage) {
      return fromMessage;
    }

    return null;
  }

  if (typeof value !== 'object') {
    return null;
  }

  const record = value as Record<string, unknown>;
  if (typeof record.type === 'string') {
    return {
      type: record.type,
      payload: record.payload,
    };
  }

  const entries = Object.entries(record);
  if (entries.length !== 1) {
    return null;
  }

  const [type, payload] = entries[0];
  return { type, payload };
}

function formatStructuredPayload(error: StructuredError): string | null {
  switch (error.type) {
    case 'App':
    case 'Collection':
    case 'Corridor':
    case 'Pin':
    case 'Metadata': {
      const nested = normalizeStructuredError(error.payload);
      return nested ? formatStructuredPayload(nested) : null;
    }
    case 'Validation':
    case 'Db':
    case 'Io':
    case 'Internal':
    case 'Security':
    case 'NotFound':
      return typeof error.payload === 'string' ? error.payload : error.type;
    case 'PathBusy':
      return typeof error.payload === 'object' && error.payload
        ? `Path is busy and cannot be renamed right now: ${String((error.payload as Record<string, unknown>).path ?? '')}`
        : 'Path is busy and cannot be renamed right now';
    case 'RuntimePathNotFound':
      return typeof error.payload === 'object' && error.payload
        ? `Runtime path not found: ${String((error.payload as Record<string, unknown>).target ?? '')}`
        : 'Runtime path not found';
    case 'DuplicateName':
      return typeof error.payload === 'object' && error.payload
        ? `Collection name '${String((error.payload as Record<string, unknown>).name ?? '')}' already exists in this corridor`
        : 'Collection name already exists in this corridor';
    case 'MissingMods':
      return typeof error.payload === 'object' && error.payload
        ? `Missing mods on disk: ${String((error.payload as Record<string, unknown>).count ?? 0)} mod(s) not found`
        : 'Missing mods on disk';
    case 'NoUndoAvailable':
      return 'No undo state is available for this corridor';
    case 'CannotModifyUndoSnapshot':
      return 'Undo snapshots are no longer supported';
    case 'NoModsPath':
      return typeof error.payload === 'object' && error.payload
        ? `Game '${String((error.payload as Record<string, unknown>).game_id ?? '')}' has no mods path configured`
        : 'Game has no mods path configured';
    case 'GameNotFound':
      return typeof error.payload === 'object' && error.payload
        ? `Game '${String((error.payload as Record<string, unknown>).game_id ?? '')}' not found`
        : 'Game not found';
    case 'CorridorMismatch':
      return typeof error.payload === 'object' && error.payload
        ? `Cannot apply ${(error.payload as Record<string, unknown>).collection_mode ?? 'this'} collection while in ${(error.payload as Record<string, unknown>).current_mode ?? 'current'} corridor`
        : 'Collection corridor does not match current mode';
    case 'RenameFailed':
      return typeof error.payload === 'object' && error.payload
        ? `Rename failed for '${String((error.payload as Record<string, unknown>).path ?? '')}': ${String((error.payload as Record<string, unknown>).error ?? '')}`
        : 'Rename failed';
    case 'PartialRenameFailed':
      return typeof error.payload === 'object' && error.payload
        ? `Batch rename partially failed: ${String((error.payload as Record<string, unknown>).succeeded ?? 0)} succeeded, ${String((error.payload as Record<string, unknown>).failed ?? 0)} failed`
        : 'Batch rename partially failed';
    case 'FileInUse':
      return typeof error.payload === 'object' && error.payload
        ? `File is in use: ${String((error.payload as Record<string, unknown>).path ?? '')}`
        : 'File is in use by another process';
    default:
      return null;
  }
}

export function extractFileInUsePayload(error: unknown): FileInUsePayload | null {
  const structured = normalizeStructuredError(error);
  if (!structured || structured.type !== 'FileInUse') {
    return null;
  }

  if (!structured.payload || typeof structured.payload !== 'object') {
    return null;
  }

  const payload = structured.payload as Record<string, unknown>;
  if (typeof payload.path !== 'string' || !Array.isArray(payload.processes)) {
    return null;
  }

  const processes = payload.processes.filter(
    (entry): entry is string => typeof entry === 'string',
  );

  return {
    path: payload.path,
    processes,
  };
}

export function extractMissingModsPayload(error: unknown): MissingModsPayload | null {
  const structured = normalizeStructuredError(error);
  if (!structured || structured.type !== 'MissingMods') {
    return null;
  }

  if (!structured.payload || typeof structured.payload !== 'object') {
    return null;
  }

  const payload = structured.payload as Record<string, unknown>;
  if (!Array.isArray(payload.paths)) {
    return null;
  }

  const paths = payload.paths.filter((entry): entry is string => typeof entry === 'string');
  const countValue = payload.count;
  const count =
    typeof countValue === 'number' && Number.isFinite(countValue) ? countValue : paths.length;

  return {
    count,
    paths,
  };
}

export function formatAppError(error: unknown): string {
  const structured = normalizeStructuredError(error);
  const structuredMessage = structured ? formatStructuredPayload(structured) : null;
  if (structuredMessage) {
    return structuredMessage;
  }

  if (error instanceof Error && error.message) {
    return error.message;
  }

  if (typeof error === 'string') {
    return error;
  }

  return 'Unexpected error';
}
