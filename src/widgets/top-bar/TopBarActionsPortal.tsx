import { createPortal } from 'react-dom';
import { useEffect, useState, type ReactNode } from 'react';

interface TopBarActionsPortalProps {
  children: ReactNode;
}

export function TopBarActionsPortal({ children }: TopBarActionsPortalProps) {
  const [target, setTarget] = useState<HTMLElement | null>(null);

  useEffect(() => {
    setTarget(document.getElementById('topbar-actions-portal'));
  }, []);

  return target ? createPortal(children, target) : null;
}
