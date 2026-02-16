/**
 * ContextMenu — Radix UI-powered context menu with DaisyUI styling.
 * Replaces the old custom Portal-based implementation.
 */

import { ReactElement, ReactNode } from 'react';
import {
  Root,
  Trigger,
  Portal,
  Content,
  Item,
  Separator,
  Sub,
  SubTrigger,
  SubContent,
} from '@radix-ui/react-context-menu';
import { LucideIcon, ChevronRight } from 'lucide-react';

/* ── Root + Trigger + Content (convenience wrapper) ────────────── */

interface ContextMenuProps {
  children: ReactElement;
  content: ReactNode;
}

export function ContextMenu({ children, content }: ContextMenuProps) {
  return (
    <Root modal={false}>
      <Trigger asChild>{children}</Trigger>
      <Portal>
        <Content
          collisionPadding={8}
          onCloseAutoFocus={(event) => event.preventDefault()}
          className="context-menu-content min-w-[180px] rounded-lg border border-base-content/10 bg-base-100 shadow-xl text-sm p-1 z-50
            data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95
            data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95"
        >
          {content}
        </Content>
      </Portal>
    </Root>
  );
}

/* ── Item ─────────────────────────────────────────────────────── */

interface ContextMenuItemProps {
  children: ReactNode;
  onClick?: () => void;
  danger?: boolean;
  disabled?: boolean;
  icon?: LucideIcon;
}

export function ContextMenuItem({
  children,
  onClick,
  danger = false,
  disabled = false,
  icon: Icon,
}: ContextMenuItemProps) {
  return (
    <Item
      disabled={disabled}
      onSelect={onClick}
      className={`flex items-center gap-2 px-3 py-1.5 rounded-md text-sm outline-none cursor-default select-none transition-colors
        ${
          danger
            ? 'text-error data-highlighted:bg-error/10'
            : 'text-base-content data-highlighted:bg-base-content/8'
        }
        ${disabled ? 'opacity-40 pointer-events-none' : ''}
      `}
    >
      {Icon && <Icon size={14} className={danger ? 'text-error' : 'text-base-content/70'} />}
      <span className="flex-1 text-left">{children}</span>
    </Item>
  );
}

/* ── Separator ────────────────────────────────────────────────── */

export function ContextMenuSeparator() {
  return <Separator className="my-1 h-px bg-base-content/10" />;
}

/* ── Sub-menu ─────────────────────────────────────────────────── */

interface ContextMenuSubProps {
  label: ReactNode;
  children: ReactNode;
  icon?: LucideIcon;
}

export function ContextMenuSub({ label, children, icon: Icon }: ContextMenuSubProps) {
  return (
    <Sub>
      <SubTrigger
        className="flex items-center gap-2 px-3 py-1.5 rounded-md text-sm outline-none cursor-default select-none text-base-content
          data-highlighted:bg-base-content/8 data-[state=open]:bg-base-content/5"
      >
        {Icon && <Icon size={14} className="text-base-content/70" />}
        <span className="flex-1">{label}</span>
        <ChevronRight size={12} className="opacity-50" />
      </SubTrigger>
      <Portal>
        <SubContent
          collisionPadding={8}
          className="min-w-[160px] rounded-lg border border-base-content/10 bg-base-100 shadow-xl text-sm p-1 z-50
            data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95
            data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95"
        >
          {children}
        </SubContent>
      </Portal>
    </Sub>
  );
}
