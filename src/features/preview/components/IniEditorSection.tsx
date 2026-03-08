import {
  Edit2,
  ExternalLink,
  Keyboard,
  TriangleAlert,
  Hash,
  ToggleRight,
  MapPin,
} from 'lucide-react';
import { useState } from 'react';
import { open } from '@tauri-apps/plugin-shell';
import type { KeyBindSectionGroup, ModFeatureSummary, HashSummary } from '../previewPanelUtils';
import { AdvancedKeybindModal } from './AdvancedKeybindModal';
import { useIniConflicts } from '../hooks/usePreviewData';
import { useAppStore } from '../../../stores/useAppStore';
import { useQueryClient } from '@tanstack/react-query';
import { folderKeys } from '../../../hooks/useFolders';
import type { FolderGridResponse, ModFolder } from '../../../types/mod';

export type IniEditorTab = 'keybind' | 'information';

interface IniEditorSectionProps {
  activePath: string | null;
  activeObjectName?: string;
  selectedFolderName?: string;
  activeTab: IniEditorTab;
  sections: KeyBindSectionGroup[];
  openSectionIds: Set<string>;
  draftByField: Record<string, string>;
  fieldErrors: Record<string, string>;
  hashSummaries: HashSummary[];
  modFeatureSummaries: ModFeatureSummary[];
  conflictingKeys: Set<string>;
  editorDirty: boolean;
  isSaving: boolean;
  onTabChange: (tab: IniEditorTab) => void;
  onToggleSection: (sectionId: string) => void;
  onFieldChange: (fieldId: string, value: string) => void;
  onSave: () => Promise<boolean | void> | void;
  onDiscard: () => void;
}

export default function IniEditorSection({
  activePath,
  activeObjectName,
  selectedFolderName,
  activeTab,
  sections,
  openSectionIds,
  draftByField,
  fieldErrors,
  hashSummaries,
  modFeatureSummaries,
  conflictingKeys,
  editorDirty,
  isSaving,
  onTabChange,
  onToggleSection,
  onFieldChange,
  onSave,
  onDiscard,
}: IniEditorSectionProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [advancedKeybindFieldId, setAdvancedKeybindFieldId] = useState<string | null>(null);

  const { data: conflicts } = useIniConflicts(
    activePath,
    activeTab === 'information' && hashSummaries.length > 0,
  );

  const queryClient = useQueryClient();
  const setSelectedObjectType = useAppStore((state) => state.setSelectedObjectType);
  const setSelectedObject = useAppStore((state) => state.setSelectedObject);
  const setGridSelection = useAppStore((state) => state.setGridSelection);
  const setActivePane = useAppStore((state) => state.setActivePane);

  // Manual save mode: no auto-save timer.

  const handleCheckLocation = (folderPath: string) => {
    // Find the folder in the react-query cache
    const allQueries = queryClient.getQueriesData<FolderGridResponse>({ queryKey: folderKeys.all });
    let targetFolder: ModFolder | undefined;

    for (const [, data] of allQueries) {
      if (!data) continue;
      targetFolder = data.children.find((f) => f.path === folderPath);
      if (targetFolder) break;
    }

    if (!targetFolder) {
      console.warn('Folder not found in cache', folderPath);
      return;
    }

    // Navigate to it
    if (targetFolder.category) {
      setSelectedObjectType(targetFolder.category);
    }
    if (targetFolder.object_id) {
      setSelectedObject(targetFolder.object_id);
    }
    if (targetFolder.id) {
      setGridSelection(new Set([targetFolder.id]));
    }
    setActivePane('folderGrid');
  };

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

      <div role="tablist" className="tabs tabs-box mb-2 w-fit bg-base-200/70 p-0.5 text-xs">
        <button
          role="tab"
          className={`tab h-7 min-h-7 px-3 text-xs ${activeTab === 'keybind' ? 'tab-active' : ''}`}
          onClick={() => onTabChange('keybind')}
        >
          Key Bind
        </button>
        <button
          role="tab"
          className={`tab h-7 min-h-7 px-3 text-xs ${activeTab === 'information' ? 'tab-active' : ''}`}
          onClick={() => onTabChange('information')}
        >
          Information
        </button>
      </div>

      {activeTab === 'keybind' ? (
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
                                            <TriangleAlert
                                              size={14}
                                              className="text-warning ml-1"
                                            />
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
      ) : (
        <div className="flex flex-col gap-4">
          {/* Mod Features List */}
          <div className="space-y-3">
            <h4 className="flex items-center gap-2 text-sm font-bold text-base-content/70 px-1 border-b border-base-content/10 pb-2">
              <ToggleRight size={16} className="text-primary" />
              Mod Features
            </h4>

            {modFeatureSummaries.length === 0 ? (
              <div className="text-xs text-base-content/50 px-2 italic">
                No interactive features detected.
              </div>
            ) : (
              <div className="grid grid-cols-1 gap-2">
                {modFeatureSummaries.map((feature) => (
                  <div
                    key={feature.featureName}
                    className="flex flex-col gap-1.5 p-3 rounded-lg border border-base-content/10 bg-base-200/30"
                  >
                    <div className="flex justify-between items-start gap-4">
                      <div className="font-mono text-sm font-bold text-primary break-all">
                        {feature.featureName}
                      </div>
                      <div className="badge badge-sm badge-ghost font-mono opacity-80 whitespace-nowrap">
                        {feature.statesCount} States
                      </div>
                    </div>
                    {feature.triggerKeys.length > 0 ? (
                      <div className="flex flex-wrap items-center gap-1.5 mt-1">
                        <span className="text-[10px] uppercase font-bold tracking-wider text-base-content/50">
                          Toggle Keys:
                        </span>
                        {feature.triggerKeys.map((key) => (
                          <kbd
                            key={key}
                            className="kbd kbd-xs bg-base-100 border-base-content/20 text-[10px] break-all"
                          >
                            {key}
                          </kbd>
                        ))}
                      </div>
                    ) : (
                      <div className="text-[10px] uppercase font-bold tracking-wider text-base-content/30 mt-1">
                        No assigned keys
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Textured / Shader Overrides and Conflicts */}
          <div className="space-y-3 mt-2">
            <h4 className="flex items-center gap-2 text-sm font-bold text-base-content/70 px-1 border-b border-base-content/10 pb-2">
              <Hash size={16} className="text-secondary" />
              Textured / Shader Overrides
            </h4>

            {hashSummaries.length === 0 ? (
              <div className="text-xs text-base-content/50 px-2 italic">No overrides found.</div>
            ) : (
              <div className="flex flex-col gap-2">
                {hashSummaries.map((hashItem, idx) => {
                  const conflict = conflicts?.find((c) => c.hash === hashItem.hash);
                  const otherPaths = conflict
                    ? conflict.mod_paths.filter((p) => p !== activePath)
                    : [];
                  const isConflicting = otherPaths.length > 0;

                  return (
                    <div
                      key={`${hashItem.hash}-${idx}`}
                      className={`flex flex-col gap-2 p-3 border rounded-lg transition-colors ${
                        isConflicting
                          ? 'bg-error/10 border-error/30'
                          : 'bg-base-200 border-base-content/10'
                      }`}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex items-center gap-2 overflow-hidden">
                          <span
                            className={`font-mono text-xs font-bold badge badge-outline ${isConflicting ? 'text-error border-error' : 'text-secondary'}`}
                          >
                            {hashItem.hash}
                          </span>
                          <span
                            className="text-xs text-base-content/70 font-mono truncate"
                            title={hashItem.sectionName}
                          >
                            {hashItem.sectionName}
                          </span>
                        </div>
                        {isConflicting && (
                          <div className="flex items-center gap-1 text-[10px] uppercase font-bold tracking-wider text-error">
                            <TriangleAlert size={12} />
                            Conflict Detected
                          </div>
                        )}
                      </div>

                      {isConflicting && (
                        <div className="flex flex-col gap-1.5 mt-1 border-t border-error/20 pt-2">
                          <span className="text-[10px] text-base-content/70">
                            Conflicts with these enabled mods:
                          </span>
                          {otherPaths.map((p) => {
                            const name = p.split(/[/\\]/).pop();
                            return (
                              <div
                                key={p}
                                className="flex items-center justify-between gap-2 overflow-hidden bg-base-100/50 rounded p-1.5 border border-base-content/5"
                              >
                                <span className="text-xs font-medium truncate" title={p}>
                                  {name}
                                </span>
                                <button
                                  className="btn btn-xs btn-ghost text-primary shrink-0 hover:bg-primary hover:text-primary-content transition-colors"
                                  onClick={() => handleCheckLocation(p)}
                                  title="Jump to this mod in the Object List"
                                >
                                  <MapPin size={12} />
                                  Check Location
                                </button>
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      )}

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
