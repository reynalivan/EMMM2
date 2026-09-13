import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { LiquidGlass, type LiquidGlassProps } from 'quick-liquid/react';
import { cn } from '@/shared/lib/utils';
import { type LiquidRole, useLiquidThemeConfig } from './liquidTheme';

type LiquidSurfaceProps = Omit<LiquidGlassProps, 'active' | 'config' | 'liquidPress' | 'children'> &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> & {
    children: ReactNode;
    liquidRole: LiquidRole;
    interactive?: boolean;
    contentClassName?: string;
  };

/** Shared QuickLiquid host. Content remains native semantic markup inside the glass layer. */
export function LiquidSurface({
  children,
  liquidRole,
  interactive = false,
  className,
  contentClassName,
  ...props
}: LiquidSurfaceProps) {
  const { config, prefersReducedMotion } = useLiquidThemeConfig(liquidRole);

  return (
    <LiquidGlass
      {...props}
      active
      config={config}
      liquidPress={interactive && !prefersReducedMotion ? { scale: 0.985, squish: 0.01 } : false}
      className={cn('liquid-surface', `liquid-surface--${liquidRole}`, className)}
    >
      <div className={cn('liquid-surface__content', contentClassName)}>{children}</div>
    </LiquidGlass>
  );
}
