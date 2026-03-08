import React, { useState, useEffect, useRef } from 'react';
import { Keyboard, Check, X, Lightbulb } from 'lucide-react';
import { mapBrowserKeyTo3DMigoto } from './keyMapper';

export interface AdvancedKeybindModalProps {
  isOpen: boolean;
  onClose: () => void;
  onApply: (formattedKey: string) => void;
  initialValue?: string;
  objectName?: string;
  folderName?: string;
}

export const AdvancedKeybindModal: React.FC<AdvancedKeybindModalProps> = ({
  isOpen,
  onClose,
  onApply,
  initialValue = '',
  objectName,
  folderName,
}) => {
  const [isListening, setIsListening] = useState(true);

  const lowerInitial = initialValue.toLowerCase();
  const [ctrlKey, setCtrlKey] = useState(
    lowerInitial.includes('ctrl') && !lowerInitial.includes('no_ctrl'),
  );
  const [altKey, setAltKey] = useState(
    lowerInitial.includes('alt') && !lowerInitial.includes('no_alt'),
  );
  const [shiftKey, setShiftKey] = useState(
    lowerInitial.includes('shift') && !lowerInitial.includes('no_shift'),
  );

  const [noCtrl, setNoCtrl] = useState(lowerInitial.includes('no_ctrl'));
  const [noAlt, setNoAlt] = useState(lowerInitial.includes('no_alt'));
  const [noShift, setNoShift] = useState(lowerInitial.includes('no_shift'));

  const [mainKey, setMainKey] = useState<string | null>(() => {
    const parts = initialValue
      .split(/\s+/)
      .filter(
        (p) => !['ctrl', 'alt', 'shift', 'no_ctrl', 'no_alt', 'no_shift'].includes(p.toLowerCase()),
      );
    return parts.length > 0 ? parts[parts.length - 1].toUpperCase() : null;
  });

  const dialogRef = useRef<HTMLDialogElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  // Handle modal open/close
  useEffect(() => {
    if (isOpen) {
      if (dialogRef.current && !dialogRef.current.open) {
        dialogRef.current.showModal();
      }
    } else {
      if (dialogRef.current && dialogRef.current.open) {
        dialogRef.current.close();
      }
    }
  }, [isOpen]);

  // Focus the overlay when listening
  useEffect(() => {
    if (isListening && overlayRef.current && isOpen) {
      overlayRef.current.focus();
    }
  }, [isListening, isOpen]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (!isListening) return;

    e.preventDefault();
    e.stopPropagation();

    const mapped = mapBrowserKeyTo3DMigoto(e);

    setCtrlKey(e.ctrlKey);
    setAltKey(e.altKey);
    setShiftKey(e.shiftKey);

    // Default to strict modifiers if the actual modifier wasn't pressed
    setNoCtrl(!e.ctrlKey);
    setNoAlt(!e.altKey);
    setNoShift(!e.shiftKey);

    if (mapped) {
      setMainKey(mapped);
      setIsListening(false); // Stop listening once a full key combo is pressed
    } else {
      setMainKey(null); // Just modifiers so far
    }
  };

  const handleCancel = () => {
    onClose();
  };

  const handleApply = () => {
    if (!mainKey) {
      // Nothing captured
      onClose();
      return;
    }

    const parts = [];
    if (ctrlKey) parts.push('ctrl');
    if (noCtrl && !ctrlKey) parts.push('no_ctrl');

    if (altKey) parts.push('alt');
    if (noAlt && !altKey) parts.push('no_alt');

    if (shiftKey) parts.push('shift');
    if (noShift && !shiftKey) parts.push('no_shift');

    parts.push(mainKey);
    onApply(parts.join(' '));
    onClose();
  };

  // Prevent closing when clicking inside the modal box
  const handleDialogClick = (e: React.MouseEvent) => {
    if (e.target === dialogRef.current) {
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <dialog
      ref={dialogRef}
      className="modal bg-base-300/60 backdrop-blur-sm"
      onClick={handleDialogClick}
      onClose={onClose}
    >
      <div className="modal-box p-0 max-w-lg overflow-hidden border border-base-content/10 shadow-2xl bg-base-200">
        <div className="bg-base-300 px-6 py-4 flex flex-col gap-3 border-b border-base-content/10">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Keyboard className="text-primary" size={20} />
              <h3 className="font-bold text-lg">Advanced Key Capture</h3>
            </div>
            <button className="btn btn-sm btn-circle btn-ghost" onClick={handleCancel}>
              <X size={16} />
            </button>
          </div>
          <div className="flex flex-col gap-1 mt-1">
            {(objectName || folderName) && (
              <div className="flex items-center gap-2 text-sm">
                {objectName && <span className="font-bold text-primary">{objectName}</span>}
                {objectName && folderName && (
                  <span className="text-base-content/20 font-light">/</span>
                )}
                {folderName && (
                  <span className="text-xs font-mono bg-base-100 px-2 py-0.5 rounded border border-base-content/10 text-base-content/70">
                    {folderName}
                  </span>
                )}
              </div>
            )}

            <div className="flex items-center gap-2 text-xs">
              <span className="text-base-content/50">Original Key:</span>
              {initialValue ? (
                <kbd className="font-mono bg-base-200/50 px-1.5 py-0.5 rounded text-base-content/80 font-medium tracking-wide">
                  {initialValue}
                </kbd>
              ) : (
                <span className="text-base-content/30 italic">Not set</span>
              )}
            </div>
          </div>
        </div>

        <div className="p-6 flex flex-col gap-6">
          {/* Capture Area */}
          <div
            ref={overlayRef}
            tabIndex={0}
            onKeyDown={handleKeyDown}
            onClick={() => setIsListening(true)}
            className={`
              relative w-full h-32 rounded-xl flex items-center justify-center outline-none cursor-pointer
              transition-all duration-200 border-2
              ${
                isListening
                  ? 'bg-base-100 border-primary shadow-[0_0_15px_-3px_rgba(var(--p),0.4)] ring-4 ring-primary/20'
                  : 'bg-base-100/50 border-base-content/20 hover:border-primary/50'
              }
            `}
          >
            {isListening ? (
              <div className="flex flex-col items-center gap-2 text-primary animate-pulse">
                <Keyboard size={32} />
                <span className="font-bold tracking-wide">PRESS KEY COMBINATION...</span>
              </div>
            ) : (
              <div className="flex flex-col items-center gap-2 text-base-content/70">
                <span className="text-sm font-medium">
                  Captured. Click to recapture or edit below.
                </span>
              </div>
            )}
          </div>

          {/* Result Preview */}
          <div className="flex flex-wrap items-center justify-center gap-2 min-h-12 bg-base-300/40 p-4 rounded-lg font-mono text-lg font-bold">
            {ctrlKey && <kbd className="kbd text-primary border-primary/30">CTRL</kbd>}
            {altKey && <kbd className="kbd text-accent border-accent/30">ALT</kbd>}
            {shiftKey && <kbd className="kbd text-secondary border-secondary/30">SHIFT</kbd>}

            {mainKey ? (
              <kbd className="kbd bg-base-100 text-white shadow-sm border-base-content/30 px-4">
                {mainKey}
              </kbd>
            ) : (
              <span className="text-base-content/30 font-medium">No Key</span>
            )}
          </div>

          {/* Modifier Toggles */}
          <div className="bg-base-100 rounded-lg p-4 border border-base-content/10">
            <div className="flex items-center gap-2 text-sm font-medium mb-3 text-base-content/80">
              <Lightbulb size={16} className="text-warning" />
              <span>Strict Modifiers (3DMigoto)</span>
            </div>

            <p className="text-xs text-base-content/60 mb-4 leading-relaxed">
              By default, a keybind triggers even if extra modifiers are pressed. Check these to{' '}
              <b>prevent</b> triggering when the modifier is held down.
            </p>

            <div className="flex items-center gap-6">
              <label
                className={`flex gap-3 items-center cursor-pointer transition-opacity ${shiftKey ? 'opacity-40 grayscale pointer-events-none' : 'hover:opacity-80'}`}
                title={shiftKey ? 'Shift is part of the combo, cannot restrict.' : ''}
              >
                <input
                  type="checkbox"
                  className="checkbox checkbox-sm checkbox-primary"
                  checked={noShift && !shiftKey}
                  onChange={(e) => setNoShift(e.target.checked)}
                  disabled={shiftKey}
                />
                <span className="text-sm font-medium">no_shift</span>
              </label>

              <label
                className={`flex gap-3 items-center cursor-pointer transition-opacity ${ctrlKey ? 'opacity-40 grayscale pointer-events-none' : 'hover:opacity-80'}`}
                title={ctrlKey ? 'Ctrl is part of the combo, cannot restrict.' : ''}
              >
                <input
                  type="checkbox"
                  className="checkbox checkbox-sm checkbox-primary"
                  checked={noCtrl && !ctrlKey}
                  onChange={(e) => setNoCtrl(e.target.checked)}
                  disabled={ctrlKey}
                />
                <span className="text-sm font-medium">no_ctrl</span>
              </label>

              <label
                className={`flex gap-3 items-center cursor-pointer transition-opacity ${altKey ? 'opacity-40 grayscale pointer-events-none' : 'hover:opacity-80'}`}
                title={altKey ? 'Alt is part of the combo, cannot restrict.' : ''}
              >
                <input
                  type="checkbox"
                  className="checkbox checkbox-sm checkbox-primary"
                  checked={noAlt && !altKey}
                  onChange={(e) => setNoAlt(e.target.checked)}
                  disabled={altKey}
                />
                <span className="text-sm font-medium">no_alt</span>
              </label>
            </div>
          </div>
        </div>

        <div className="bg-base-300 px-6 py-4 flex items-center justify-end gap-3 border-t border-base-content/10">
          <button className="btn btn-ghost" onClick={handleCancel}>
            Cancel
          </button>
          <button className="btn btn-primary min-w-25" onClick={handleApply} disabled={!mainKey}>
            <Check size={18} />
            Apply
          </button>
        </div>
      </div>
    </dialog>
  );
};
