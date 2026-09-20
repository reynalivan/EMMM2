import type { LiquidGlassConfig } from 'quick-liquid';
import { useQuery } from '@tanstack/react-query';
import { useSyncExternalStore } from 'react';
import {
  commands,
  type AppSettings,
  type CustomTheme,
  type LiquidRoleConfig,
} from '@/shared/api/tauri/bindings';
import { usePrefersReducedMotion } from '@/shared/lib/hooks/usePrefersReducedMotion';
import { normalizeThemeSetting, resolveTheme } from '@/shared/lib/themeOptions';

export const LIQUID_ROLES = ['nav', 'control', 'indicator', 'overlay'] as const;

export type LiquidRole = (typeof LIQUID_ROLES)[number];

const SETTINGS_QUERY_KEY = ['settings'] as const;
const CUSTOM_THEME_QUERY_KEY = (id: string) => ['custom-themes', 'detail', id] as const;
const DARK_QUERY = '(prefers-color-scheme: dark)';

function subscribeToColorScheme(onChange: () => void) {
  const media = window.matchMedia(DARK_QUERY);
  media.addEventListener('change', onChange);
  return () => media.removeEventListener('change', onChange);
}

function useLiquidResolvedTheme(): string {
  const settings = useQuery<AppSettings>({
    queryKey: SETTINGS_QUERY_KEY,
    queryFn: () => commands.getSettings(),
    staleTime: Infinity,
  });
  const prefersDark = useSyncExternalStore(
    subscribeToColorScheme,
    () => window.matchMedia(DARK_QUERY).matches,
  );
  return resolveTheme(normalizeThemeSetting(settings.data?.theme), prefersDark);
}

function useLiquidCustomTheme(id: string | null) {
  return useQuery<CustomTheme>({
    queryKey: CUSTOM_THEME_QUERY_KEY(id ?? ''),
    queryFn: () => commands.loadCustomTheme(id as string),
    enabled: Boolean(id),
    staleTime: Infinity,
  });
}

const ROLE_CONFIG: Record<LiquidRole, Partial<LiquidGlassConfig>> = {
  nav: {
    material: 'regular',
    tint: 'transparent',
    tintOpacity: 0,
    blur: 24,
    saturation: 1.08,
    refractionStrength: 14,
    bezelWidth: 18,
    chromaticAberration: 0,
    edgeHighlight: 0.12,
    specularStrength: 0.04,
    elevation: 0,
    borderRadius: 0,
    quality: 'high',
  },
  control: {
    material: 'thin',
    blur: 10,
    refractionStrength: 14,
    bezelWidth: 22,
    chromaticAberration: 0.13,
    edgeHighlight: 0.68,
    specularStrength: 0.2,
    quality: 'high',
  },
  indicator: {
    material: 'clear',
    blur: 4,
    refractionStrength: 12,
    bezelWidth: 18,
    chromaticAberration: 0.1,
    edgeHighlight: 0.64,
    specularStrength: 0.16,
    quality: 'high',
  },
  overlay: {
    material: 'regular',
    blur: 24,
    saturation: 1.08,
    tint: 'transparent',
    tintOpacity: 0,
    refractionStrength: 14,
    bezelWidth: 20,
    chromaticAberration: 0,
    edgeHighlight: 0.12,
    specularStrength: 0.04,
    elevation: 0.35,
    borderRadius: 14,
    quality: 'high',
  },
};

function customRoleConfig(
  theme: CustomTheme | undefined,
  role: LiquidRole,
): LiquidRoleConfig | undefined {
  return theme?.config.liquid?.[role] ?? undefined;
}

function toEngineConfig(config: LiquidRoleConfig | undefined): Partial<LiquidGlassConfig> {
  if (!config) return {};

  return {
    material: (config.material ?? undefined) as LiquidGlassConfig['material'],
    tint: resolveLiquidTint(config.tint ?? undefined),
    tintOpacity: config.tint_opacity ?? undefined,
    blur: config.blur ?? undefined,
    refractionStrength: config.refraction_strength ?? undefined,
    bezelWidth: config.bezel_width ?? undefined,
    chromaticAberration: config.chromatic_aberration ?? undefined,
    edgeHighlight: config.edge_highlight ?? undefined,
    specularStrength: config.specular_strength ?? undefined,
    lightAngle: config.light_angle ?? undefined,
    quality: (config.quality ?? undefined) as LiquidGlassConfig['quality'],
    appearance: (config.appearance ?? undefined) as LiquidGlassConfig['appearance'],
  };
}

const RGB_TINT = /^\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*$/;

function rgbList(value: string): string | null {
  const match = RGB_TINT.exec(value);
  if (!match) return null;

  const channels = match.slice(1).map(Number);
  return channels.every((channel) => channel <= 255) ? channels.join(', ') : null;
}

function computedRgb(value: string): string | null {
  if (typeof document === 'undefined') return null;

  const probe = document.createElement('span');
  probe.style.color = value;
  if (!probe.style.color) return null;

  document.body.appendChild(probe);
  const resolved = getComputedStyle(probe).color;
  probe.remove();
  const match = /^rgba?\((\d+),\s*(\d+),\s*(\d+)/.exec(resolved);
  return match ? `${match[1]}, ${match[2]}, ${match[3]}` : null;
}

/** Converts theme CSS colours to the RGB triplet required by QuickLiquid. */
export function resolveLiquidTint(tint?: string): string {
  if (!tint) return 'var(--liquid-tint-rgb)';
  return rgbList(tint) ?? computedRgb(tint) ?? 'var(--liquid-tint-rgb)';
}

function defaultAppearance(theme: string): NonNullable<LiquidGlassConfig['appearance']> {
  if (theme === 'light') return 'light';
  if (theme === 'onyx') return 'dark';
  return 'auto';
}

export function useLiquidThemeConfig(role: LiquidRole): {
  config: Partial<LiquidGlassConfig>;
  prefersReducedMotion: boolean;
} {
  const theme = useLiquidResolvedTheme();
  const customTheme = useLiquidCustomTheme(theme === 'onyx' || theme === 'light' ? null : theme);
  const prefersReducedMotion = usePrefersReducedMotion();
  const overrides = customRoleConfig(customTheme.data, role);

  const config: Partial<LiquidGlassConfig> = {
    appearance: defaultAppearance(theme),
    tint: 'transparent',
    tintOpacity: 0,
    lightAngle: -35,
    hoverLighting: !prefersReducedMotion && role !== 'overlay',
    cursorTracking: !prefersReducedMotion && role === 'nav',
    dynamicLighting: !prefersReducedMotion && role === 'nav',
    parallax: false,
    ...ROLE_CONFIG[role],
    ...toEngineConfig(overrides),
  };

  if (prefersReducedMotion) {
    config.cursorTracking = false;
    config.dynamicLighting = false;
    config.hoverLighting = false;
    config.parallax = false;
  }

  if (role === 'nav') {
    config.borderRadius = 0;
    config.cursorTracking = false;
    config.dynamicLighting = false;
    config.hoverLighting = false;
  }

  return { config, prefersReducedMotion };
}
