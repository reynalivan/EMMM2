import { useEffect, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

interface TopBarActionsPortalProps {
  children: ReactNode;
}

/** Renders page or feature actions into the app shell's top-bar action slot. */
export function TopBarActionsPortal({ children }: TopBarActionsPortalProps) {
  const [target, setTarget] = useState<HTMLElement | null>(null);

  useEffect(() => {
    setTarget(document.getElementById('topbar-actions-portal'));
  }, []);

  return target ? createPortal(children, target) : null;
}
