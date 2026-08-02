import { useEffect, type RefObject } from 'react';

/**
 * Keeps a native `<dialog>` in sync with React state.
 *
 * Both calls are guarded against the dialog's current state on purpose:
 * `showModal()` on an already-open dialog throws InvalidStateError, and
 * `close()` on a closed one fires a spurious `close` event.
 */
export function useDialogSync(dialogRef: RefObject<HTMLDialogElement | null>, open: boolean): void {
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }

    if (open && !dialog.open) {
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [dialogRef, open]);
}
