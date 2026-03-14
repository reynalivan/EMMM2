import { Edit2, ExternalLink, Keyboard, TriangleAlert } from 'lucide-react';
import { useState } from 'react';
import { open } from '@tauri-apps/plugin-shell';
import type { KeyBindSectionGroup } from '../previewPanelUtils';
import { AdvancedKeybindModal } from './AdvancedKeybindModal';
interface IniEditorSectionProps {
  activePath: string | null;
  activeObjectName?: string;
  selectedFolderName?: string;
  sections: KeyBindSectionGroup[];
  openSectionIds: Set<string>;
  draftByField: Record<string, string>;
  fieldErrors: Record<string, string>;
  conflictingKeys: Set<string>;
  editorDirty: boolean;
  isSaving: boolean;
  onToggleSection: (sectionId: string) => void;
  onFieldChange: (fieldId: string, value: string) => void;
  onSave: () => Promise<boolean | void> | void;
  onDiscard: () => void;
}

export default function IniEditorSection({
  activePath,
  activeObjectName,
  selectedFolderName,
  sections,
  openSectionIds,
  draftByField,
  fieldErrors,
  conflictingKeys,
  editorDirty,
  isSaving,
  onToggleSection,
  onFieldChange,
  onSave,
  onDiscard,
}: IniEditorSectionProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [advancedKeybindFieldId, setAdvancedKeybindFieldId] = useState<string | null>(null);

  // Manual save mode: no auto-save timer.

  return (
    <div className="mb-6 relative">
      <div className="sticky top-17 z-10 -mx-6 mb-2 flex items-center justify-between bg-base-100/95 px-6 py-2 backdrop-blur-sm border-b border-base-content/10">
        <h3 className="text-xs font-bold uppercase tracking-widest text-white/40">INI Editor</h3>
        <div className="flex items-center gap-2">
          {!isEditing ? (
            <button
              className="btn btn-ghost btn-xs"
              onClick={() => setIsEditing(true)}
              title="Edit keybinds"
            >
              <Edit2 size={13} />
              Edit
            </button>
          ) : !editorDirty && !isSaving ? (
            <button
              className="btn btn-ghost btn-xs text-base-content/70"
              onClick={() => setIsEditing(false)}
              title="Close edit mode"
            >
              Close
            </button>
          ) : (
            <>
              <button
                className="btn btn-ghost btn-xs text-error"
                disabled={isSaving}
                onClick={() => {
                  onDiscard();
                  setIsEditing(false);
                }}
                title="Discard INI changes"
              >
                Revert
              </button>
              <button
                className="btn btn-primary btn-xs"
                disabled={isSaving || Object.keys(fieldErrors).length > 0}
                onClick={async () => {
                  await onSave();
                  setIsEditing(false);
                }}
                title="Save INI changes"
              >
                Save
              </button>
            </>
          )}
        </div>
      </div>

      <div className="space-y-4" onDoubleClick={() => setIsEditing(true)}>
        {sections.length === 0 && (
          <div className="text-xs text-base-content/50">No key binding sections detected.</div>
        )}

        {sections.map((fileGroup) => {
          const isOpen = openSectionIds.has(fileGroup.id);
          return (
            <div
              key={fileGroup.id}
              className="rounded-lg border border-base-content/10 overflow-hidden"
            >
              <div className="flex w-full items-center bg-base-200/50 px-3 py-2">
                <div
                  className="flex flex-1 items-center justify-between cursor-pointer group"
                  onClick={() => onToggleSection(fileGroup.id)}
                >
                  <div className="flex items-center">
                    <span className="text-sm font-bold text-base-content/90 mr-2 group-hover:text-primary transition-colors">
                      {fileGroup.fileName}
                    </span>
                    <button
                      className="btn btn-ghost btn-xs px-1 hover:bg-base-content/10"
                      title="Open in default editor"
                      onClick={(e) => {
                        e.stopPropagation();
                        if (activePath) open(`${activePath}\\${fileGroup.fileName}`);
                      }}
                    >
                      <ExternalLink size={12} className="opacity-70" />
                    </button>
                  </div>
                  <div className="text-[11px] text-base-content/50 pr-2 flex items-center gap-1.5 font-mono">
                    <span>{fileGroup.rangeLabel}</span>
                    <span className="text-[8px] opacity-70">{isOpen ? '▲' : '▼'}</span>
                  </div>
                </div>
              </div>

              {isOpen && (
                <div className="flex flex-col bg-base-100/10">
                  {fileGroup.sections.map((section, idx) => (
                    <div
                      key={`${fileGroup.id}-${section.sectionName}-${idx}`}
                      className={`space-y-2 p-3 ${idx > 0 ? 'border-t border-base-content/10' : ''}`}
                    >
                      <div className="text-sm font-bold text-primary mb-2">
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
                                  className={`flex items-center gap-3 w-full px-3 py-1.5 rounded-lg border transition-all ${
                                    isPrimary
                                      ? 'bg-primary/5 border-primary/30 shadow-sm'
                                      : 'bg-base-200/30 border-base-content/10'
                                  } ${fieldErrors[field.id] ? 'border-error!' : ''}`}
                                  title={fieldErrors[field.id]}
                                >
                                  <span
                                    className={`text-xs font-mono w-12 text-left select-none ${isPrimary ? 'text-white font-bold' : 'text-base-content/60'}`}
                                  >
                                    {field.label}
                                  </span>
                                  <div className="flex-1 flex items-center gap-1 group/input">
                                    <input
                                      type="text"
                                      className={`input input-xs w-full h-7 px-3 font-mono uppercase ${
                                        isPrimary
                                          ? 'input-primary bg-base-100/90 font-bold shadow-inner text-white'
                                          : 'input-bordered bg-base-100/50'
                                      } ${fieldErrors[field.id] ? 'input-error bg-error/10' : ''}`}
                                      value={currentValue}
                                      onChange={(e) =>
                                        onFieldChange(field.id, e.target.value.toUpperCase())
                                      }
                                      placeholder={`Enter ${field.label}...`}
                                    />
                                    {isPrimary &&
                                      conflictingKeys.has(currentValue.trim().toUpperCase()) && (
                                        <div
                                          className="tooltip tooltip-left"
                                          data-tip="Warning: This key is used in multiple sections."
                                        >
                                          <TriangleAlert size={14} className="text-warning ml-1" />
                                        </div>
                                      )}
                                    <button
                                      type="button"
                                      className={`btn btn-xs btn-square ${isPrimary ? 'btn-ghost text-white hover:bg-white/20' : 'btn-ghost text-base-content/50 hover:text-primary hover:bg-primary/10'}`}
                                      title="Auto Detect Key Combination"
                                      onClick={() => setAdvancedKeybindFieldId(field.id)}
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
                                className={`flex items-center gap-3 w-full px-4 py-1.5 rounded-lg border transition-all ${
                                  isPrimary
                                    ? 'bg-base-200/50 border-primary/10'
                                    : 'bg-base-100 border-transparent'
                                }`}
                              >
                                <span
                                  className={`text-xs font-mono w-12 text-left select-none ${isPrimary ? 'text-white font-bold' : 'text-base-content/50'}`}
                                >
                                  {field.label}
                                </span>
                                <div className="flex-1 min-w-0 flex items-center gap-2">
                                  <kbd
                                    className={`kbd kbd-sm min-h-6 h-auto py-1 px-3 whitespace-normal break-all text-left leading-tight bg-base-100 shadow-sm ${
                                      isPrimary
                                        ? conflictingKeys.has(currentValue.trim().toUpperCase())
                                          ? 'border-warning/50 text-warning font-bold'
                                          : 'border-base-content/15 text-white'
                                        : 'border-base-content/15 text-base-content'
                                    }`}
                                  >
                                    {field.value}
                                  </kbd>
                                  {isPrimary &&
                                    conflictingKeys.has(currentValue.trim().toUpperCase()) && (
                                      <div
                                        className="tooltip tooltip-left"
                                        data-tip="Key Conflict! Used in multiple sections."
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
                                  <span className="opacity-50 mr-1">{field.label}:</span>
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
