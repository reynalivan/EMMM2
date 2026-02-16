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
  id: string;
  fileName: string;
  sectionName: string;
  fields: KeyBindEditableField[];
  rangeLabel: string;
}

export interface VariableOccurrenceInfo {
  fileName: string;
  sectionName: string;
  lineIdx: number;
  value: string;
}

export interface VariableInfoSummary {
  name: string;
  count: number;
  minValue: number | null;
  maxValue: number | null;
  occurrences: VariableOccurrenceInfo[];
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

function toRangeLabel(fields: KeyBindEditableField[]): string {
  if (fields.length === 0) {
    return '0 entries';
  }
  return fields.length === 1 ? '1 entry' : `${fields.length} entries`;
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

    for (const [sectionName, fields] of groupsBySection.entries()) {
      const sortedFields = [...fields].sort((a, b) => a.lineIdx - b.lineIdx);
      groups.push({
        id: `${entry.fileName}:${sectionName}`,
        fileName: entry.fileName,
        sectionName,
        fields: sortedFields,
        rangeLabel: toRangeLabel(sortedFields),
      });
    }
  }

  return groups.sort((a, b) => {
    const byFile = a.fileName.localeCompare(b.fileName, undefined, { sensitivity: 'base' });
    if (byFile !== 0) {
      return byFile;
    }
    return a.sectionName.localeCompare(b.sectionName, undefined, { sensitivity: 'base' });
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

export function buildVariableInfoSummaries(
  documents: Array<{ fileName: string; document: IniDocumentLike | null | undefined }>,
): VariableInfoSummary[] {
  const byName = new Map<string, VariableInfoSummary>();

  for (const entry of documents) {
    const document = entry.document;
    if (!document || document.mode !== 'Structured') {
      continue;
    }

    const sectionByLine = buildSectionByLine(document.raw_lines);

    for (const variable of document.variables) {
      const sectionName = sectionByLine.get(variable.line_idx) ?? 'Global';
      const parsed = Number(variable.value);
      const valueNumber = Number.isFinite(parsed) ? parsed : null;

      const existing = byName.get(variable.name);
      if (!existing) {
        byName.set(variable.name, {
          name: variable.name,
          count: 1,
          minValue: valueNumber,
          maxValue: valueNumber,
          occurrences: [
            {
              fileName: entry.fileName,
              sectionName,
              lineIdx: variable.line_idx,
              value: variable.value,
            },
          ],
        });
        continue;
      }

      existing.count += 1;
      existing.occurrences.push({
        fileName: entry.fileName,
        sectionName,
        lineIdx: variable.line_idx,
        value: variable.value,
      });

      if (valueNumber !== null) {
        existing.minValue =
          existing.minValue === null ? valueNumber : Math.min(existing.minValue, valueNumber);
        existing.maxValue =
          existing.maxValue === null ? valueNumber : Math.max(existing.maxValue, valueNumber);
      }
    }
  }

  return Array.from(byName.values()).sort((a, b) => a.name.localeCompare(b.name));
}
