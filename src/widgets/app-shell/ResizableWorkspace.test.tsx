import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ResizableWorkspace from './ResizableWorkspace';

const fixture = vi.hoisted(() => ({
  isMobile: false,
  state: {
    leftPanelWidth: 260,
    rightPanelWidth: 320,
    setPanelWidths: vi.fn(),
    mobileActivePane: 'grid',
    isPreviewOpen: true,
  },
}));

vi.mock('@/app/store', () => ({
  useAppStore: <T,>(selector: (state: typeof fixture.state) => T) => selector(fixture.state),
}));
vi.mock('@/shared/lib/hooks/useResponsive', () => ({
  useResponsive: () => ({ isMobile: fixture.isMobile }),
}));

const workspace = <ResizableWorkspace leftPanel={null} mainPanel={null} rightPanel={null} />;

describe('ResizableWorkspace gesture cleanup', () => {
  beforeEach(() => {
    fixture.isMobile = false;
    fixture.state.setPanelWidths.mockClear();
    document.body.style.cursor = 'crosshair';
    document.body.style.userSelect = 'text';
  });

  afterEach(() => {
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  });

  it.each(['left', 'right'])('restores previous body styles when the %s resize ends', (side) => {
    render(workspace);
    fireEvent.mouseDown(screen.getByTestId(`resize-handle-${side}`));
    expect(document.body.style.cursor).toBe('col-resize');
    expect(document.body.style.userSelect).toBe('none');

    fireEvent.mouseUp(window);
    fireEvent.mouseUp(window);

    expect(document.body.style.cursor).toBe('crosshair');
    expect(document.body.style.userSelect).toBe('text');
    expect(fixture.state.setPanelWidths).toHaveBeenCalledExactlyOnceWith(260, 320);
  });

  it('restores body styles and removes gesture listeners on unmount', () => {
    document.body.style.setProperty('cursor', 'crosshair', 'important');
    document.body.style.setProperty('user-select', 'text', 'important');
    const view = render(workspace);
    fireEvent.mouseDown(screen.getByTestId('resize-handle-left'));
    view.unmount();
    fireEvent.mouseUp(window);

    expect(document.body.style.cursor).toBe('crosshair');
    expect(document.body.style.userSelect).toBe('text');
    expect(document.body.style.getPropertyPriority('cursor')).toBe('important');
    expect(document.body.style.getPropertyPriority('user-select')).toBe('important');
    expect(fixture.state.setPanelWidths).not.toHaveBeenCalled();
  });

  it('ends the gesture when the window loses focus', () => {
    render(workspace);
    fireEvent.mouseDown(screen.getByTestId('resize-handle-left'));
    fireEvent.blur(window);

    expect(document.body.style.cursor).toBe('crosshair');
    expect(document.body.style.userSelect).toBe('text');
    fireEvent.mouseUp(window);
    expect(fixture.state.setPanelWidths).toHaveBeenCalledTimes(1);
  });

  it('cancels a desktop gesture when the workspace becomes mobile', () => {
    const view = render(workspace);
    fireEvent.mouseDown(screen.getByTestId('resize-handle-left'));
    fixture.isMobile = true;
    view.rerender(<ResizableWorkspace leftPanel={null} mainPanel={null} rightPanel={null} />);

    expect(document.body.style.cursor).toBe('crosshair');
    expect(document.body.style.userSelect).toBe('text');
    fireEvent.mouseUp(window);
    expect(fixture.state.setPanelWidths).not.toHaveBeenCalled();

    fixture.isMobile = false;
    view.rerender(<ResizableWorkspace leftPanel={null} mainPanel={null} rightPanel={null} />);
    expect(document.body.style.cursor).toBe('crosshair');
  });

  it('preserves body properties changed by another owner during the gesture', () => {
    const view = render(workspace);
    fireEvent.mouseDown(screen.getByTestId('resize-handle-left'));
    document.body.style.cursor = 'wait';
    document.body.style.userSelect = 'all';
    view.unmount();

    expect(document.body.style.cursor).toBe('wait');
    expect(document.body.style.userSelect).toBe('all');
  });
});
