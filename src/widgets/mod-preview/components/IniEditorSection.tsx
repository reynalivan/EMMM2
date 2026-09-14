import { ChevronDown, ChevronUp, Edit2, ExternalLink, Keyboard, TriangleAlert } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from '@/shared/ui/toast';
import { commands } from '../../../shared/api/tauri/bindings';
import { formatAppError } from '../../../shared/lib/appError';
import type { KeyBindSectionGroup } from '../utils/previewPanelUtils';
import { AdvancedKeybindModal } from './AdvancedKeybindModal';
interface IniEditorSectionProps {
  activePath: string | null;
  activeGameId: string | null;
  activeObjectName?: string;
  selectedFolderName?: string;
  sections: KeyBindSectionGroup[];
  openSectionIds: Set<string>;
  draftByField: Record<string, string>;
  fieldErrors: Record<string, string>;
  conflictingKeys: Set<string>;
  editorDirty: boolean;
  isSaving: boolean;
  canEdit?: boolean;
  onToggleSection: (sectionId: string) => void;
  onFieldChange: (fieldId: string, value: string) => void;
  onSave: () => Promise<boolean | void> | void;
  onDiscard: () => void;
}

export default function IniEditorSection({
  activePath,
  activeGameId,
  activeObjectName,
  selectedFolderName,
  sections,
  openSectionIds,
  draftByField,
  fieldErrors,
  conflictingKeys,
  editorDirty,
  isSaving,
  canEdit = true,
  onToggleSection,
  onFieldChange,
  onSave,
  onDiscard,
}: IniEditorSectionProps) {
  const { t } = useTranslation(['preview']);
  const [isEditing, setIsEditing] = useState(false);
  const [advancedKeybindFieldId, setAdvancedKeybindFieldId] = useState<string | null>(null);
  const shouldStickToolbar = isEditing || editorDirty;

  useEffect(() => {
    if (!canEdit && isEditing) {
      setIsEditing(false);
    }
  }, [canEdit, isEditing]);

  return (
    <div className="mb-6 relative">
      <div
        className={`${
          shouldStickToolbar
            ? 'glass-surface sticky top-[calc(var(--workspace-topbar-height)+var(--preview-header-height))] z-10 -mx-6 px-6'
            : 'bg-transparent'
        } mb-2 flex items-center justify-between border-b border-base-content/10 py-2`}
      >
        <h3 className="text-sm font-semibold text-base-content/80">
          {t('preview:ini_editor.title')}
        </h3>
        <div className="flex items-center gap-2">
          {!isEditing ? (
            <button
              className="btn btn-ghost btn-xs"
              onClick={() => setIsEditing(true)}
              title={t('preview:ini_editor.edit_keybinds')}
              disabled={!canEdit}
            >
              <Edit2 size={13} />
              {t('preview:actions.edit')}
            </button>
          ) : !editorDirty && !isSaving ? (
            <button
              className="btn btn-ghost btn-xs text-base-content/70"
              onClick={() => setIsEditing(false)}
              title={t('preview:ini_editor.close_edit')}
            >
              {t('preview:actions.close')}
            </button>
          ) : (
            <>
              <button
                className="btn btn-ghost btn-xs text-error"
                disabled={isSaving || !canEdit}
                onClick={() => {
                  onDiscard();
                  setIsEditing(false);
                }}
                title={t('preview:ini_editor.revert_changes')}
              >
                {t('preview:actions.revert')}
              </button>
              <button
                className="btn btn-primary btn-xs"
                disabled={isSaving || !canEdit || Object.keys(fieldErrors).length > 0}
                onClick={async () => {
                  await onSave();
                  setIsEditing(false);
                }}
                title={t('preview:ini_editor.save_changes')}
              >
                {t('preview:actions.save')}
              </button>
            </>
          )}
        </div>
      </div>

      <div className="space-y-2">
        {sections.length === 0 && (
          <div className="text-xs text-base-content/50">{t('preview:ini_editor.no_sections')}</div>
        )}

        {sections.map((fileGroup) => {
          const isOpen = openSectionIds.has(fileGroup.id);
          const fields = fileGroup.sections.flatMap((section) => section.fields);
          const bindingCount = fields.filter(
            (field) => field.label === 'key' || field.label === 'back',
          ).length;
          const changedCount = editorDirty
            ? fields.filter((field) => (draftByField[field.id] ?? field.value) !== field.value)
                .length
            : 0;
          return (
            <div
              key={fileGroup.id}
              className="overflow-hidden rounded-md border border-base-content/10"
            >
              <div className="flex w-full items-center bg-base-200/35 px-3 py-2">
                <button
                  type="button"
                  className="group flex min-w-0 flex-1 items-center justify-between text-left"
                  onClick={() => onToggleSection(fileGroup.id)}
                  aria-expanded={isOpen}
                >
                  <div className="flex min-w-0 items-center">
                    <span className="mr-2 truncate text-sm font-semibold text-base-content/90 transition-colors group-hover:text-primary">
                      {fileGroup.fileName}
                    </span>
                  </div>
                  <div className="ml-2 flex shrink-0 items-center gap-1.5 pr-2 text-[10px] text-base-content/50">
                    <span className="hidden sm:inline">{fileGroup.rangeLabel}</span>
                    <span>{t('preview:ini_editor.binding_count', { count: bindingCount })}</span>
                    {changedCount > 0 && (
                      <span className="rounded bg-primary/10 px-1 py-0.5 text-primary">
                        {t('preview:ini_editor.unsaved_count', { count: changedCount })}
                      </span>
                    )}
                    {isOpen ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                  </div>
                </button>
                <button
                  type="button"
                  className="btn btn-ghost btn-xs ml-1 shrink-0 px-1 hover:bg-base-content/10"
                  title={t('preview:ini_editor.open_in_editor')}
                  aria-label={t('preview:ini_editor.open_in_editor')}
                  onClick={() => {
                    if (activeGameId && activePath && canEdit) {
                      void commands
                        .openIniInEditor(activeGameId, activePath, fileGroup.fileName)
                        .catch((error) => {
                          toast.error(
                            t('preview:errors.open_location_failed', {
                              error: formatAppError(error),
                            }),
                          );
                        });
                    }
                  }}
                  disabled={!canEdit || !activeGameId}
                >
                  <ExternalLink size={12} className="opacity-70" />
                </button>
              </div>

              {isOpen && (
                <div className="flex flex-col">
                  {fileGroup.sections.map((section, idx) => (
                    <div
                      key={`${fileGroup.id}-${section.sectionName}-${idx}`}
                      className={`space-y-2 px-3 py-2.5 ${idx > 0 ? 'border-t border-base-content/10' : ''}`}
                    >
                      <div className="mb-1.5 text-xs font-semibold text-base-content/70">
                        {section.sectionName}
                      </div>
                      <div className="flex flex-col gap-2">
                        {/* Part 1: Full-Width Editable Fields (Inline Flex) */}
                        {section.fields
                          .filter((f) => f.label === 'key' || f.label === 'back')
                          .map((field) => {
                            const isPrimary = field.label === 'key';
                            const currentValue = draftByField[field.id] ?? field.value;

                            if (isEditing) {
                              return (
                                <div
                                  key={field.id}
                                  className={`flex w-full items-center gap-3 rounded-md border px-3 py-1.5 transition-[background-color,border-color] duration-150 ${
                                    isPrimary
                                      ? 'bg-primary/5 border-primary/30 shadow-sm'
                                      : 'bg-base-200/30 border-base-content/10'
                                  } ${fieldErrors[field.id] ? 'border-error!' : ''}`}
                                  title={fieldErrors[field.id]}
                                >
                                  <span
                                    className={`text-xs font-mono w-12 text-left select-none ${isPrimary ? 'text-primary font-bold' : 'text-base-content/60'}`}
                                  >
                                    {field.label}
                                  </span>
                                  <div className="flex-1 flex items-center gap-1 group/input">
                                    <input
                                      type="text"
                                      className={`input input-xs w-full h-7 px-3 font-mono uppercase ${
                                        isPrimary
                                          ? 'input-primary bg-base-100/90 font-bold shadow-inner text-base-content'
                                          : 'input-bordered bg-base-100/50'
                                      } ${fieldErrors[field.id] ? 'input-error bg-error/10' : ''}`}
                                      value={currentValue}
                                      onChange={(e) =>
                                        onFieldChange(field.id, e.target.value.toUpperCase())
                                      }
                                      disabled={!canEdit}
                                      placeholder={t('preview:ini_editor.enter_field', {
                                        label: field.label,
                                      })}
                                    />
                                    {isPrimary &&
                                      conflictingKeys.has(currentValue.trim().toUpperCase()) && (
                                        <div
                                          className="tooltip tooltip-left"
                                          data-tip={t('preview:ini_editor.conflict_warning')}
                                        >
                                          <TriangleAlert size={14} className="text-warning ml-1" />
                                        </div>
                                      )}
                                    <button
                                      type="button"
                                      className={`btn btn-xs btn-square ${isPrimary ? 'btn-ghost text-primary hover:bg-primary/20' : 'btn-ghost text-base-content/50 hover:text-primary hover:bg-primary/10'}`}
                                      title={t('preview:ini_editor.auto_detect')}
                                      onClick={() => setAdvancedKeybindFieldId(field.id)}
                                      disabled={!canEdit}
                                    >
                                      <Keyboard size={14} />
                                    </button>
                                  </div>
                                </div>
                              );
                            }

                            // Read-Only View for Editable Fields
                            return (
                              <div
                                key={field.id}
                                className={`flex w-full items-center gap-3 rounded-md border px-3 py-1.5 transition-[background-color,border-color] duration-150 ${
                                  isPrimary
                                    ? 'bg-primary/5 border-primary/15'
                                    : 'border-transparent bg-transparent'
                                }`}
                              >
                                <span
                                  className={`text-xs font-mono w-12 text-left select-none ${isPrimary ? 'text-primary font-bold' : 'text-base-content/50'}`}
                                >
                                  {field.label}
                                </span>
                                <div className="flex-1 min-w-0 flex items-center gap-2">
                                  <kbd
                                    className={`kbd kbd-sm min-h-6 h-auto py-1 px-3 whitespace-normal break-all text-left leading-tight bg-base-100 shadow-sm ${
                                      isPrimary
                                        ? conflictingKeys.has(currentValue.trim().toUpperCase())
                                          ? 'border-warning/50 text-warning font-bold'
                                          : 'border-base-content/15 text-base-content'
                                        : 'border-base-content/15 text-base-content'
                                    }`}
                                  >
                                    {field.value}
                                  </kbd>
                                  {isPrimary &&
                                    conflictingKeys.has(currentValue.trim().toUpperCase()) && (
                                      <div
                                        className="tooltip tooltip-left"
                                        data-tip={t('preview:ini_editor.conflict_tooltip')}
                                      >
                                        <TriangleAlert
                                          size={14}
                                          className="text-warning shrink-0"
                                        />
                                      </div>
                                    )}
                                </div>
                              </div>
                            );
                          })}

                        {/* Part 2: Compact Meta Chips Footer */}
                        {section.fields.filter((f) => f.label !== 'key' && f.label !== 'back')
                          .length > 0 && (
                          <div className="flex flex-wrap items-center gap-1 mt-0 pt-0">
                            {section.fields
                              .filter((f) => f.label !== 'key' && f.label !== 'back')
                              .map((field) => (
                                <div
                                  key={field.id}
                                  className="flex items-center px-1.5 py-1 rounded-sm text-[10px] font-mono bg-base-300/30 border border-base-content/5 text-base-content/60 leading-tight"
                                >
                                  <span className="text-muted mr-1">{field.label}:</span>
                                  <span className="break-all font-medium text-base-content/80">
                                    {field.value}
                                  </span>
                                </div>
                              ))}
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {advancedKeybindFieldId && (
        <AdvancedKeybindModal
          isOpen={true}
          initialValue={draftByField[advancedKeybindFieldId] || ''}
          objectName={activeObjectName}
          folderName={selectedFolderName}
          onClose={() => setAdvancedKeybindFieldId(null)}
          onApply={(keyStr) => {
            onFieldChange(advancedKeybindFieldId, keyStr.toUpperCase());
            setAdvancedKeybindFieldId(null);
          }}
        />
      )}
    </div>
  );
}
