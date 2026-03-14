import { validateKeyBinding } from './keybindingValidator';

export interface IniVariableLike {
  name: string;
  value: string;
  line_idx: number;
}

export interface KeyBindingLike {
  section_name: string;
  key: string | null;
  back: string | null;
  key_line_idx: number | null;
  back_line_idx: number | null;
}

export interface IniDocumentLike {
  raw_lines: string[];
  mode: 'Structured' | 'RawFallback';
  variables: IniVariableLike[];
  key_bindings: KeyBindingLike[];
}

export interface KeyBindEditableField {
  id: string;
  fileName: string;
  sectionName: string;
  lineIdx: number;
  label: string;
  prefix: string;
  value: string;
}

export interface KeyBindSectionGroup {
  id: string; // usually the fileName
  fileName: string;
  sections: {
    sectionName: string;
    fields: KeyBindEditableField[];
  }[];
  rangeLabel: string;
}

export interface IniWritePayload {
  updatesByFile: Record<string, { line_idx: number; content: string }[]>;
  fieldErrors: Record<string, string>;
}

function isKeySection(sectionName: string): boolean {
  return sectionName.toLowerCase().startsWith('key');
}

function buildSectionByLine(rawLines: string[]): Map<number, string> {
  const sectionByLine = new Map<number, string>();
  let currentSection = 'Global';

  for (let index = 0; index < rawLines.length; index += 1) {
    const line = rawLines[index]?.trim() ?? '';
    const sectionMatch = line.match(/^\s*\[([^\]]+)\]\s*$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1].trim();
    }
    sectionByLine.set(index, currentSection);
  }

  return sectionByLine;
}

export function shouldLoadGalleryImage(
  index: number,
  currentIndex: number,
  total: number,
): boolean {
  if (total <= 0 || index < 0 || index >= total) {
    return false;
  }

  if (Math.abs(index - currentIndex) <= 1) {
    return true;
  }

  return (currentIndex === 0 && index === total - 1) || (currentIndex === total - 1 && index === 0);
}

export function validateIniDraftValue(value: string): string | null {
  if (!value.trim()) {
    return 'Value cannot be empty.';
  }

  if (value.includes('\n') || value.includes('\r')) {
    return 'Value must be a single line.';
  }

  return null;
}

export function buildKeyBindSections(
  documents: Array<{ fileName: string; document: IniDocumentLike | null | undefined }>,
): KeyBindSectionGroup[] {
  const groups: KeyBindSectionGroup[] = [];

  for (const entry of documents) {
    const document = entry.document;
    if (!document || document.mode !== 'Structured') {
      continue;
    }

    const sectionByLine = buildSectionByLine(document.raw_lines);
    const groupsBySection = new Map<string, KeyBindEditableField[]>();

    const pushField = (sectionName: string, field: KeyBindEditableField) => {
      const bucket = groupsBySection.get(sectionName);
      if (bucket) {
        bucket.push(field);
        return;
      }
      groupsBySection.set(sectionName, [field]);
    };

    for (const binding of document.key_bindings) {
      if (!isKeySection(binding.section_name)) {
        continue;
      }

      if (binding.key_line_idx !== null && binding.key !== null) {
        pushField(binding.section_name, {
          id: `${entry.fileName}:${binding.section_name}:key:${binding.key_line_idx}`,
          fileName: entry.fileName,
          sectionName: binding.section_name,
          lineIdx: binding.key_line_idx,
          label: 'key',
          prefix: 'key =',
          value: binding.key,
        });
      }

      if (binding.back_line_idx !== null && binding.back !== null) {
        pushField(binding.section_name, {
          id: `${entry.fileName}:${binding.section_name}:back:${binding.back_line_idx}`,
          fileName: entry.fileName,
          sectionName: binding.section_name,
          lineIdx: binding.back_line_idx,
          label: 'back',
          prefix: 'back =',
          value: binding.back,
        });
      }
    }

    for (let lineIdx = 0; lineIdx < document.raw_lines.length; lineIdx += 1) {
      const sectionName = sectionByLine.get(lineIdx) ?? 'Global';
      if (!isKeySection(sectionName)) {
        continue;
      }

      const line = document.raw_lines[lineIdx];
      const assignMatch = line.match(/^\s*([A-Za-z_$][A-Za-z0-9_.$]*)\s*=\s*([^;#\r\n]+)\s*$/);
      if (!assignMatch) {
        continue;
      }

      const left = assignMatch[1].trim();
      const right = assignMatch[2].trim();
      const lowerLeft = left.toLowerCase();
      if (lowerLeft === 'key' || lowerLeft === 'back') {
        continue;
      }

      pushField(sectionName, {
        id: `${entry.fileName}:${sectionName}:assign:${lineIdx}`,
        fileName: entry.fileName,
        sectionName,
        lineIdx,
        label: left,
        prefix: `${left} =`,
        value: right,
      });
    }

    // Re-structure the groups for this document by `fileName`
    const fileSections: { sectionName: string; fields: KeyBindEditableField[] }[] = [];
    let totalKeysInFile = 0;

    for (const [sectionName, fields] of groupsBySection.entries()) {
      const sortedFields = [...fields].sort((a, b) => a.lineIdx - b.lineIdx);
      fileSections.push({
        sectionName,
        fields: sortedFields,
      });
      totalKeysInFile += sortedFields.filter((f) => f.label === 'key').length;
    }

    // Sort sections alphabetically within the file
    fileSections.sort((a, b) =>
      a.sectionName.localeCompare(b.sectionName, undefined, { sensitivity: 'base' }),
    );

    if (fileSections.length > 0) {
      groups.push({
        id: entry.fileName, // The ID of the group is the file name
        fileName: entry.fileName,
        sections: fileSections,
        rangeLabel:
          totalKeysInFile === 0
            ? '0 keys'
            : totalKeysInFile === 1
              ? '1 key'
              : `${totalKeysInFile} keys`,
      });
    }
  }

  return groups.sort((a, b) => {
    return a.fileName.localeCompare(b.fileName, undefined, { sensitivity: 'base' });
  });
}

export function toFieldValueMap(fields: KeyBindEditableField[]): Record<string, string> {
  return fields.reduce<Record<string, string>>((acc, field) => {
    acc[field.id] = field.value;
    return acc;
  }, {});
}

export function toIniWritePayload(
  fields: KeyBindEditableField[],
  draftByField: Record<string, string>,
  initialByField: Record<string, string>,
): IniWritePayload {
  const fieldErrors: Record<string, string> = {};
  const updatesByFile: Record<string, { line_idx: number; content: string }[]> = {};

  for (const field of fields) {
    const draft = draftByField[field.id] ?? '';
    const initial = initialByField[field.id] ?? '';
    if (draft === initial) {
      continue;
    }

    const error = validateIniDraftValue(draft);
    if (error) {
      fieldErrors[field.id] = error;
      continue;
    }

    // Key/back fields require keybinding-specific validation
    if (field.label === 'key' || field.label === 'back') {
      const kbError = validateKeyBinding(draft);
      if (kbError) {
        fieldErrors[field.id] = kbError;
        continue;
      }
    }

    const updates = updatesByFile[field.fileName] ?? [];
    updates.push({
      line_idx: field.lineIdx,
      content: `${field.prefix} ${draft}`,
    });
    updatesByFile[field.fileName] = updates;
  }

  return {
    updatesByFile,
    fieldErrors,
  };
}

export function getConflictingKeys(
  keyBindSections: KeyBindSectionGroup[],
  draftByField: Record<string, string>,
): Set<string> {
  const keyUsage = new Map<string, string[]>();

  for (const group of keyBindSections) {
    for (const section of group.sections) {
      const keyField = section.fields.find((f) => f.label === 'key');
      if (keyField) {
        const rawValue = draftByField[keyField.id] ?? keyField.value;
        const normalized = rawValue.trim().toLowerCase();

        if (normalized) {
          const users = keyUsage.get(normalized) || [];
          users.push(section.sectionName);
          keyUsage.set(normalized, users);
        }
      }
    }
  }

  const conflictingKeys = new Set<string>();
  for (const [key, users] of keyUsage.entries()) {
    // Only report true conflicts if it's used in 2 or more distinct sections
    const distinctSections = new Set(users);
    if (distinctSections.size > 1) {
      conflictingKeys.add(key.toUpperCase());
    }
  }

  return conflictingKeys;
}
