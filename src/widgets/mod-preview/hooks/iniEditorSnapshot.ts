import {
  buildKeyBindSections,
  toFieldValueMap,
  type IniDocumentLike,
  type KeyBindEditableField,
  type KeyBindSectionGroup,
} from '../utils/previewPanelUtils';

export interface IniEditorBaseline {
  sessionKey: string;
  sections: KeyBindSectionGroup[];
  values: Record<string, string>;
  rawLinesByFile: Record<string, string[]>;
}

export interface IniEditorSnapshot {
  sections: KeyBindSectionGroup[];
  rawLinesByFile: Record<string, string[]>;
  readVersion: number;
}

export interface IniDocumentEntry {
  fileName: string;
  document: IniDocumentLike | null | undefined;
}

export function fieldsFromSections(sections: KeyBindSectionGroup[]): KeyBindEditableField[] {
  return sections.flatMap((group) => group.sections.flatMap((section) => section.fields));
}

export function createIniEditorSnapshot(
  documents: IniDocumentEntry[],
  readVersion: number,
): IniEditorSnapshot {
  const sections = buildKeyBindSections(documents);
  return {
    sections,
    rawLinesByFile: Object.fromEntries(
      documents.flatMap(({ fileName, document }) =>
        document ? [[fileName, document.raw_lines]] : [],
      ),
    ),
    readVersion,
  };
}

export function createIniEditorBaseline(
  sessionKey: string,
  snapshot: IniEditorSnapshot,
): IniEditorBaseline {
  return {
    sessionKey,
    sections: snapshot.sections,
    values: toFieldValueMap(fieldsFromSections(snapshot.sections)),
    rawLinesByFile: snapshot.rawLinesByFile,
  };
}

export function areRawLinesEqual(left: string[] | undefined, right: string[]): boolean {
  return (
    left !== undefined &&
    left.length === right.length &&
    left.every((line, index) => line === right[index])
  );
}

export function mergeIniEditorSnapshot(
  previous: IniEditorBaseline,
  latest: IniEditorSnapshot,
  settledFiles: Set<string>,
  confirmedFiles: Set<string> = settledFiles,
  submittedValues: Record<string, string> = {},
): IniEditorBaseline {
  const latestFields = fieldsFromSections(latest.sections);
  const latestValues = toFieldValueMap(
    latestFields.filter((field) => confirmedFiles.has(field.fileName)),
  );
  const values = { ...previous.values };
  for (const field of fieldsFromSections(previous.sections)) {
    if (settledFiles.has(field.fileName)) {
      values[field.id] = submittedValues[field.id] ?? previous.values[field.id] ?? '';
    }
  }
  Object.assign(values, latestValues);
  const sectionsByFile = new Map(
    latest.sections
      .filter((group) => confirmedFiles.has(group.fileName))
      .map((group) => [group.fileName, group]),
  );
  const sections = previous.sections.map((group) => {
    if (!confirmedFiles.has(group.fileName)) return group;
    const latestGroup = sectionsByFile.get(group.fileName);
    if (!latestGroup) return group;
    sectionsByFile.delete(group.fileName);
    const latestByName = new Map(
      latestGroup.sections.map((section) => [section.sectionName, section]),
    );
    const mergedSections = group.sections.map((section) => {
      const refreshed = latestByName.get(section.sectionName);
      if (!refreshed) return section;
      latestByName.delete(section.sectionName);
      const refreshedIds = new Set(refreshed.fields.map((field) => field.id));
      return {
        ...refreshed,
        fields: [
          ...refreshed.fields,
          ...section.fields.filter((field) => !refreshedIds.has(field.id)),
        ].sort((left, right) => left.lineIdx - right.lineIdx),
      };
    });
    return {
      ...latestGroup,
      sections: [...mergedSections, ...latestByName.values()],
    };
  });
  sections.push(...sectionsByFile.values());

  return {
    ...previous,
    sections,
    values,
    rawLinesByFile: {
      ...previous.rawLinesByFile,
      ...Object.fromEntries(
        Array.from(confirmedFiles, (fileName) => [fileName, latest.rawLinesByFile[fileName] ?? []]),
      ),
    },
  };
}
