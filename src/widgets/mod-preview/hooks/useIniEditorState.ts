import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { validateKeyBinding } from '../utils/keybindingValidator';
import { toast } from '@/shared/ui/toast';
import { formatAppError } from '../../../shared/lib/appError';
import { getConflictingKeys, toFieldValueMap, toIniWritePayload } from '../utils/previewPanelUtils';
import type { WriteModIniInput } from './usePreviewData';
import {
  areRawLinesEqual,
  createIniEditorBaseline,
  createIniEditorSnapshot,
  fieldsFromSections,
  mergeIniEditorSnapshot,
  type IniDocumentEntry,
  type IniEditorBaseline,
} from './iniEditorSnapshot';
import { dispatchWorkspaceRuntimeEvent } from '@/features/workspace-runtime';

interface IniSaveConfirmation {
  lines: string[];
  readVersion: number;
}

interface UseIniEditorStateOptions {
  gameId: string | null;
  activePath: string | null;
  documents: IniDocumentEntry[];
  readVersion: number;
  onWrite: (input: WriteModIniInput) => Promise<unknown>;
}

function areFieldMapsEqual(left: Record<string, string>, right: Record<string, string>): boolean {
  const leftKeys = Object.keys(left);
  const rightKeys = Object.keys(right);
  return leftKeys.length === rightKeys.length && leftKeys.every((key) => left[key] === right[key]);
}

function areStringSetsEqual(left: Set<string>, right: Set<string>): boolean {
  return left.size === right.size && Array.from(left).every((value) => right.has(value));
}

export function useIniEditorState({
  gameId,
  activePath,
  documents,
  readVersion,
  onWrite,
}: UseIniEditorStateOptions) {
  const { t } = useTranslation(['preview']);
  const sessionKey = `${gameId ?? ''}\0${activePath ?? ''}`;
  const snapshot = useMemo(
    () => createIniEditorSnapshot(documents, readVersion),
    [documents, readVersion],
  );
  const [baselineState, setBaselineState] = useState<IniEditorBaseline | null>(null);
  const [draftByField, setDraftByField] = useState<Record<string, string>>({});
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [openSectionIds, setOpenSectionIds] = useState<Set<string>>(new Set());
  const savingSessionsRef = useRef(new Set<string>());
  const sessionRef = useRef({ key: sessionKey, epoch: 0, mounted: true });
  const pendingWritesRef = useRef(new Map<string, Map<string, IniSaveConfirmation>>());
  const draftRef = useRef(draftByField);
  const snapshotRef = useRef(snapshot);
  const baseline =
    baselineState?.sessionKey === sessionKey
      ? baselineState
      : createIniEditorBaseline(sessionKey, snapshot);
  const fields = useMemo(() => fieldsFromSections(baseline.sections), [baseline.sections]);
  const currentDraftByField =
    baselineState?.sessionKey === sessionKey ? draftByField : baseline.values;
  const changedFields = useMemo(
    () =>
      fields.filter(
        (field) => (currentDraftByField[field.id] ?? '') !== (baseline.values[field.id] ?? ''),
      ),
    [fields, currentDraftByField, baseline.values],
  );
  const hasUnsavedEditorChanges = changedFields.length > 0;
  const changedIniFields = useMemo(
    () =>
      changedFields.map((field) => ({
        label: field.label || field.id,
        filename: field.fileName || 'Unknown INI',
        oldValue: baseline.values[field.id] ?? '',
        newValue: currentDraftByField[field.id] ?? '',
      })),
    [changedFields, baseline.values, currentDraftByField],
  );
  const conflictingKeys = useMemo(
    () => getConflictingKeys(baseline.sections, currentDraftByField),
    [baseline.sections, currentDraftByField],
  );

  useLayoutEffect(() => {
    if (sessionRef.current.key !== sessionKey) {
      pendingWritesRef.current.clear();
      sessionRef.current = { key: sessionKey, epoch: sessionRef.current.epoch + 1, mounted: true };
    }
    snapshotRef.current = snapshot;
    draftRef.current = draftByField;
    const pending = pendingWritesRef.current.get(sessionKey);
    if (!pending) return;
    const confirmed = new Set<string>();
    for (const [fileName, expected] of pending) {
      if (areRawLinesEqual(snapshot.rawLinesByFile[fileName], expected.lines)) {
        confirmed.add(fileName);
        pending.delete(fileName);
      } else if (readVersion > expected.readVersion) {
        pending.delete(fileName);
      }
    }
    if (pending.size === 0) pendingWritesRef.current.delete(sessionKey);
    if (confirmed.size === 0) return;
    setBaselineState((previous) =>
      previous?.sessionKey === sessionKey
        ? mergeIniEditorSnapshot(previous, snapshot, confirmed)
        : previous,
    );
  }, [sessionKey, snapshot, draftByField, readVersion]);

  useLayoutEffect(() => {
    sessionRef.current.mounted = true;
    return () => {
      sessionRef.current.mounted = false;
    };
  }, []);

  useEffect(() => {
    if (hasUnsavedEditorChanges || pendingWritesRef.current.has(sessionKey)) return;
    const nextBaseline = createIniEditorBaseline(sessionKey, snapshot);
    setBaselineState((previous) =>
      previous?.sessionKey === sessionKey &&
      previous.sections === nextBaseline.sections &&
      areFieldMapsEqual(previous.values, nextBaseline.values)
        ? previous
        : nextBaseline,
    );
    const nextValues = toFieldValueMap(fieldsFromSections(snapshot.sections));
    setDraftByField((previous) =>
      areFieldMapsEqual(previous, nextValues) ? previous : nextValues,
    );
    setFieldErrors((previous) => (Object.keys(previous).length === 0 ? previous : {}));
  }, [hasUnsavedEditorChanges, sessionKey, snapshot]);

  useEffect(() => {
    const validIds = new Set(baseline.sections.map((section) => section.id));
    setOpenSectionIds((previous) => {
      const next = new Set(Array.from(previous).filter((id) => validIds.has(id)));
      if (next.size === 0) baseline.sections.forEach((section) => next.add(section.id));
      return areStringSetsEqual(previous, next) ? previous : next;
    });
  }, [baseline.sections]);

  const saveEditor = async (): Promise<boolean> => {
    const saveSession = sessionRef.current;
    if (!activePath) return true;
    const submittedDraft = { ...currentDraftByField };
    const submittedBaseline = baseline;
    const payload = toIniWritePayload(fields, submittedDraft, baseline.values);
    setFieldErrors(payload.fieldErrors);
    if (Object.keys(payload.fieldErrors).length > 0) {
      toast.error(t('preview:ini_editor.invalid_values'));
      return false;
    }
    const updatesEntries = Object.entries(payload.updatesByFile);
    if (updatesEntries.length === 0) return true;
    if (savingSessionsRef.current.has(sessionKey)) return false;

    const expectedLinesByFile = new Map<string, string[]>();
    for (const [fileName, lineUpdates] of updatesEntries) {
      const sourceLines = submittedBaseline.rawLinesByFile[fileName];
      if (!sourceLines) {
        toast.error(
          t('preview:ini_editor.save_error', { error: `Missing source document for ${fileName}` }),
        );
        return false;
      }
      const expectedLines = [...sourceLines];
      for (const update of lineUpdates) expectedLines[update.line_idx] = update.content;
      expectedLinesByFile.set(fileName, expectedLines);
    }

    savingSessionsRef.current.add(sessionKey);
    try {
      const results = await Promise.allSettled(
        updatesEntries.map(async ([fileName, lineUpdates]) => {
          const expectedSourceHash = fields.find(
            (field) => field.fileName === fileName,
          )?.sourceHash;
          if (!expectedSourceHash) throw new Error(`Missing source fingerprint for ${fileName}`);
          await onWrite({ folderPath: activePath, fileName, expectedSourceHash, lineUpdates });
          return { fileName, readVersion: snapshotRef.current.readVersion };
        }),
      );
      if (!saveSession.mounted || sessionRef.current.epoch !== saveSession.epoch) return false;

      const savedFiles = new Set(
        results.flatMap((result) => (result.status === 'fulfilled' ? [result.value.fileName] : [])),
      );
      const latestSnapshot = snapshotRef.current;
      const confirmedFiles = new Set(
        Array.from(savedFiles).filter((fileName) =>
          areRawLinesEqual(
            latestSnapshot.rawLinesByFile[fileName],
            expectedLinesByFile.get(fileName) ?? [],
          ),
        ),
      );
      const pending =
        pendingWritesRef.current.get(sessionKey) ?? new Map<string, IniSaveConfirmation>();
      for (const result of results) {
        if (result.status !== 'fulfilled') continue;
        const { fileName, readVersion: startedAt } = result.value;
        if (!confirmedFiles.has(fileName) && latestSnapshot.readVersion > startedAt)
          pending.delete(fileName);
        else
          pending.set(fileName, {
            lines: expectedLinesByFile.get(fileName) ?? [],
            readVersion: startedAt,
          });
      }
      for (const fileName of confirmedFiles) pending.delete(fileName);
      if (pending.size > 0) pendingWritesRef.current.set(sessionKey, pending);
      else pendingWritesRef.current.delete(sessionKey);

      const submittedValues: Record<string, string> = {};
      for (const field of fields) {
        if (savedFiles.has(field.fileName))
          submittedValues[field.id] = submittedDraft[field.id] ?? baseline.values[field.id] ?? '';
      }
      setBaselineState((previous) =>
        previous?.sessionKey === sessionKey
          ? mergeIniEditorSnapshot(
              previous,
              latestSnapshot,
              savedFiles,
              confirmedFiles,
              submittedValues,
            )
          : previous,
      );

      if (results.every((result) => result.status === 'fulfilled')) {
        const newerDraft = fields.some(
          (field) => draftRef.current[field.id] !== submittedDraft[field.id],
        );
        if (newerDraft) return false;
        setFieldErrors({});
        toast.success(t('preview:ini_editor.saved'));
        return true;
      }
      const failed = results.find((result) => result.status === 'rejected');
      if (failed?.status === 'rejected') {
        toast.error(t('preview:ini_editor.save_error', { error: formatAppError(failed.reason) }));
      }
      return false;
    } catch (error) {
      if (sessionRef.current.mounted && sessionRef.current.epoch === saveSession.epoch) {
        toast.error(t('preview:ini_editor.save_error', { error: formatAppError(error) }));
      }
      return false;
    } finally {
      savingSessionsRef.current.delete(sessionKey);
    }
  };

  const discardEditor = () => {
    pendingWritesRef.current.delete(sessionKey);
    const nextBaseline = createIniEditorBaseline(sessionKey, snapshot);
    setBaselineState(nextBaseline);
    setDraftByField(nextBaseline.values);
    setFieldErrors({});
  };

  const requestToggleSection = (sectionId: string) => {
    const isOpen = openSectionIds.has(sectionId);
    if (isOpen && hasUnsavedEditorChanges) {
      dispatchWorkspaceRuntimeEvent({
        type: 'PREVIEW_TRANSITION_REQUESTED',
        target: { kind: 'collapseSection', sectionId },
      });
      return;
    }
    setOpenSectionIds((previous) => {
      const next = new Set(previous);
      if (isOpen) next.delete(sectionId);
      else next.add(sectionId);
      return next;
    });
  };

  const collapseSection = (sectionId: string) => {
    setOpenSectionIds((previous) => {
      const next = new Set(previous);
      next.delete(sectionId);
      return next;
    });
  };

  const updateEditorField = (fieldId: string, value: string) => {
    setDraftByField((previous) => ({ ...previous, [fieldId]: value }));
    const field = fields.find((entry) => entry.id === fieldId);
    if (field && (field.label === 'key' || field.label === 'back')) {
      const error = value.trim() ? validateKeyBinding(value) : null;
      setFieldErrors((previous) => {
        if (error) return { ...previous, [fieldId]: error };
        if (!previous[fieldId]) return previous;
        const next = { ...previous };
        delete next[fieldId];
        return next;
      });
      return;
    }
    setFieldErrors((previous) => {
      if (!previous[fieldId]) return previous;
      const next = { ...previous };
      delete next[fieldId];
      return next;
    });
  };

  return {
    keyBindSections: baseline.sections,
    openSectionIds,
    draftByField: currentDraftByField,
    fieldErrors,
    conflictingKeys,
    hasUnsavedEditorChanges,
    changedIniFields,
    saveEditor,
    discardEditor,
    requestToggleSection,
    collapseSection,
    updateEditorField,
  };
}
