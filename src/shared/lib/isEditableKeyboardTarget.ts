const NON_TEXT_INPUT_TYPES = new Set(['checkbox', 'button', 'submit', 'reset']);

export function isEditableKeyboardTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  const control = target.closest('input, textarea, select');
  if (control instanceof HTMLInputElement) return !NON_TEXT_INPUT_TYPES.has(control.type);
  if (control) return true;
  const editable = target.closest('[contenteditable]');
  return editable !== null && editable.getAttribute('contenteditable') !== 'false';
}
