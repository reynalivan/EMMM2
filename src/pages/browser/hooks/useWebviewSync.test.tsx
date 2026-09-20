import { render, waitFor } from '@testing-library/react';
import { useRef } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { BrowserTab } from '@/entities/browser';
import { isDemoMode } from '@/shared/lib/appMode';
import type { BrowserSurfacePresentation } from '../browserSurfacePresentation';
import { useWebviewSync } from './useWebviewSync';

const webview = vi.hoisted(() => ({
  hide: vi.fn().mockResolvedValue(undefined),
  setFocus: vi.fn().mockResolvedValue(undefined),
  setPosition: vi.fn().mockResolvedValue(undefined),
  setSize: vi.fn().mockResolvedValue(undefined),
  show: vi.fn().mockResolvedValue(undefined),
}));

const inactiveWebview = vi.hoisted(() => ({
  hide: vi.fn().mockResolvedValue(undefined),
  setFocus: vi.fn().mockResolvedValue(undefined),
  setPosition: vi.fn().mockResolvedValue(undefined),
  setSize: vi.fn().mockResolvedValue(undefined),
  show: vi.fn().mockResolvedValue(undefined),
}));

const getByLabel = vi.hoisted(() => vi.fn());

vi.mock('@tauri-apps/api/webview', () => ({
  Webview: { getByLabel },
}));

vi.mock('@/shared/lib/appMode', () => ({
  isDemoMode: false,
}));

const tab: BrowserTab = {
  id: 'browser-tab-1',
  title: 'Example',
  url: 'https://example.com',
};

const inactiveTab: BrowserTab = {
  id: 'browser-tab-2',
  title: 'Inactive',
  url: 'https://inactive.example.com',
};

function SurfaceHarness({
  presentation,
  tabs = [tab],
  activeTabId = tab.id,
}: {
  presentation: BrowserSurfacePresentation;
  tabs?: BrowserTab[];
  activeTabId?: string;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  useWebviewSync(containerRef, tabs, activeTabId, presentation);

  return <div ref={containerRef} />;
}

function setContainerRect(rect: Partial<DOMRect>) {
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
    bottom: (rect.top ?? 0) + (rect.height ?? 0),
    height: rect.height ?? 0,
    left: rect.left ?? 0,
    right: (rect.left ?? 0) + (rect.width ?? 0),
    toJSON: () => ({}),
    top: rect.top ?? 0,
    width: rect.width ?? 0,
    x: rect.left ?? 0,
    y: rect.top ?? 0,
  } as DOMRect);
}

describe('useWebviewSync', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getByLabel.mockImplementation((label: string) =>
      Promise.resolve(label === tab.id ? webview : inactiveWebview),
    );
    setContainerRect({ left: 12, top: 96, width: 800, height: 600 });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('keeps the webview visible without stealing focus while DOM browser UI is active', async () => {
    expect(isDemoMode).toBe(false);
    render(
      <SurfaceHarness presentation={{ visibility: 'visible', focus: 'dom', leftInsetPx: 240 }} />,
    );

    await waitFor(() => expect(webview.show).toHaveBeenCalledOnce());

    expect(webview.setSize).toHaveBeenCalledOnce();
    expect(webview.setPosition).toHaveBeenCalledOnce();
    expect(webview.setFocus).not.toHaveBeenCalled();
    expect(webview.hide).not.toHaveBeenCalled();
  });

  it('positions, shows, and focuses only the active webview in normal browser mode', async () => {
    render(
      <SurfaceHarness
        presentation={{ visibility: 'visible', focus: 'browser', leftInsetPx: 0 }}
        tabs={[tab, inactiveTab]}
      />,
    );

    await waitFor(() => expect(webview.setFocus).toHaveBeenCalledOnce());

    expect(webview.setSize).toHaveBeenCalledOnce();
    expect(webview.setPosition).toHaveBeenCalledOnce();
    expect(webview.show).toHaveBeenCalledOnce();
    expect(inactiveWebview.hide).toHaveBeenCalledOnce();
    expect(inactiveWebview.show).not.toHaveBeenCalled();
    expect(inactiveWebview.setFocus).not.toHaveBeenCalled();
  });

  it('keeps a live webview mounted when browser chrome takes DOM focus', async () => {
    const { rerender } = render(
      <SurfaceHarness presentation={{ visibility: 'visible', focus: 'browser', leftInsetPx: 0 }} />,
    );

    await waitFor(() => expect(webview.setFocus).toHaveBeenCalledOnce());

    vi.clearAllMocks();
    rerender(
      <SurfaceHarness presentation={{ visibility: 'visible', focus: 'dom', leftInsetPx: 0 }} />,
    );

    await waitFor(() => expect(webview.show).toHaveBeenCalledOnce());

    expect(webview.setFocus).not.toHaveBeenCalled();
    expect(webview.hide).not.toHaveBeenCalled();

    vi.clearAllMocks();
    rerender(
      <SurfaceHarness presentation={{ visibility: 'visible', focus: 'browser', leftInsetPx: 0 }} />,
    );

    await waitFor(() => expect(webview.setFocus).toHaveBeenCalledOnce());

    expect(webview.show).toHaveBeenCalledOnce();
    expect(webview.hide).not.toHaveBeenCalled();
  });

  it('hides native webviews even when the browser container has no dimensions', async () => {
    setContainerRect({ left: 0, top: 0, width: 0, height: 0 });

    render(
      <SurfaceHarness presentation={{ visibility: 'hidden', focus: 'dom', leftInsetPx: 0 }} />,
    );

    await waitFor(() => expect(webview.hide).toHaveBeenCalledOnce());

    expect(webview.show).not.toHaveBeenCalled();
    expect(webview.setSize).not.toHaveBeenCalled();
    expect(webview.setPosition).not.toHaveBeenCalled();
  });
});
