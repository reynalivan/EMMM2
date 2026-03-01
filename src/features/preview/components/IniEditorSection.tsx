import { Check } from 'lucide-react';
import type { KeyBindSectionGroup, VariableInfoSummary } from '../previewPanelUtils';

export type IniEditorTab = 'keybind' | 'information';

interface IniEditorSectionProps {
  activePath: string | null;
  activeTab: IniEditorTab;
  sections: KeyBindSectionGroup[];
  openSectionIds: Set<string>;
  draftByField: Record<string, string>;
  fieldErrors: Record<string, string>;
  variableSummaries: VariableInfoSummary[];
  editorDirty: boolean;
  isSaving: boolean;
  onTabChange: (tab: IniEditorTab) => void;
  onToggleSection: (sectionId: string) => void;
  onFieldChange: (fieldId: string, value: string) => void;
  onSave: () => void;
  onDiscard: () => void;
}

function renderVariableRange(summary: VariableInfoSummary): string {
  if (summary.minValue === null || summary.maxValue === null) {
    return 'Range: non-numeric';
  }
  if (summary.minValue === summary.maxValue) {
    return `Range: ${summary.minValue}`;
  }
  return `Range: ${summary.minValue} to ${summary.maxValue}`;
}

export default function IniEditorSection({
  activePath,
  activeTab,
  sections,
  openSectionIds,
  draftByField,
  fieldErrors,
  variableSummaries,
  editorDirty,
  isSaving,
  onTabChange,
  onToggleSection,
  onFieldChange,
  onSave,
  onDiscard,
}: IniEditorSectionProps) {
  const activeEntries =
    variableSummaries.find((item) => item.name === '$active')?.occurrences ?? [];

  return (
    <div className="mb-6">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-xs font-bold uppercase tracking-widest text-white/40">INI Editor</h3>
        <div className="flex items-center gap-2">
          <button
            className="btn btn-ghost btn-xs text-warning"
            disabled={!editorDirty || isSaving}
            onClick={onDiscard}
            title="Discard INI changes"
          >
            Discard
          </button>
          <button
            className="btn btn-primary btn-xs"
            disabled={!editorDirty || isSaving || !activePath}
            onClick={onSave}
            title="Save INI editor"
          >
            <Check size={13} />
            Save INI
          </button>
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
        <div className="space-y-2 rounded-lg border border-base-content/10 p-2">
          {sections.length === 0 && (
            <div className="text-xs text-base-content/50">No key binding sections detected.</div>
          )}

          {sections.map((section) => {
            const isOpen = openSectionIds.has(section.id);
            return (
              <div key={section.id} className="rounded-lg border border-base-content/10">
                <button
                  className="flex w-full items-center justify-between rounded-t-lg bg-base-200/50 px-3 py-2 text-left"
                  onClick={() => onToggleSection(section.id)}
                >
                  <span className="text-xs font-semibold text-base-content/80">
                    {section.fileName} / [{section.sectionName}]
                  </span>
                  <span className="text-[11px] text-base-content/50">
                    {section.rangeLabel} {isOpen ? '▲' : '▼'}
                  </span>
                </button>

                {isOpen && (
                  <div className="space-y-2 p-3">
                    {section.fields.map((field) => (
                      <div key={field.id} className="rounded-md border border-base-content/10 p-2">
                        <div className="mb-1 text-[11px] text-base-content/60">{field.label}</div>
                        <input
                          type="text"
                          aria-label={`${section.sectionName} ${field.label}`}
                          className={`input input-sm w-full ${fieldErrors[field.id] ? 'input-error' : 'input-bordered'}`}
                          value={draftByField[field.id] ?? ''}
                          onChange={(event) => onFieldChange(field.id, event.target.value)}
                        />
                        {fieldErrors[field.id] && (
                          <p className="mt-1 text-[11px] text-error">{fieldErrors[field.id]}</p>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      ) : (
        <div className="space-y-2 rounded-lg border border-base-content/10 p-2">
          {activeEntries.length > 0 && (
            <div className="rounded-lg border border-success/30 bg-success/10 p-2 text-xs text-success-content">
              <div className="font-semibold">$active overview</div>
              {activeEntries.map((entry, index) => (
                <div key={`${entry.fileName}-${entry.sectionName}-${index}`}>
                  value {entry.value} at [{entry.sectionName}] in {entry.fileName}
                </div>
              ))}
            </div>
          )}

          {variableSummaries.length === 0 && (
            <div className="text-xs text-base-content/50">
              No parsed variable information available.
            </div>
          )}

          {variableSummaries.map((summary) => (
            <div key={summary.name} className="rounded-lg border border-base-content/10 p-2">
              <div className="text-sm font-semibold">{summary.name}</div>
              <div className="text-xs text-base-content/60">
                {renderVariableRange(summary)} | occurrences: {summary.count}
              </div>
              <div className="mt-1 space-y-1 text-[11px] text-base-content/60">
                {summary.occurrences.map((entry, index) => (
                  <div key={`${summary.name}-${entry.fileName}-${entry.sectionName}-${index}`}>
                    {entry.fileName} / [{entry.sectionName}]: {entry.value}
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
