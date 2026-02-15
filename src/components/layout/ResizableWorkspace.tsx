/* eslint-disable react-hooks/set-state-in-effect */
import { ReactNode, useState, useEffect, useRef } from 'react';
import { useAppStore } from '../../stores/useAppStore';
import { useResponsive } from '../../hooks/useResponsive';

interface ResizableWorkspaceProps {
  leftPanel: ReactNode;
  mainPanel: ReactNode;
  rightPanel: ReactNode;
}

export default function ResizableWorkspace({
  leftPanel,
  mainPanel,
  rightPanel,
}: ResizableWorkspaceProps) {
  const { leftPanelWidth, rightPanelWidth, setPanelWidths, mobileActivePane, isPreviewOpen } =
    useAppStore();

  // Resize State
  const [widths, setWidths] = useState({ left: leftPanelWidth, right: rightPanelWidth });
  const [isResizing, setIsResizing] = useState<'left' | 'right' | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Mobile detection
  const { isMobile } = useResponsive();

  // Update local state when store changes
  useEffect(() => {
    setWidths({ left: leftPanelWidth, right: rightPanelWidth });
  }, [leftPanelWidth, rightPanelWidth]);

  // Ref for resize logic
  const widthsRef = useRef(widths);
  useEffect(() => {
    widthsRef.current = widths;
  }, [widths]);

  // Resize Handlers
  const handleMouseDown = (direction: 'left' | 'right') => {
    setIsResizing(direction);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  };

  useEffect(() => {
    const handleMouseUp = () => {
      if (isResizing) {
        setPanelWidths(widthsRef.current.left, widthsRef.current.right);
        setIsResizing(null);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      }
    };

    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing || !containerRef.current) return;

      const containerRect = containerRef.current.getBoundingClientRect();
      const minCategoryWidth = 220;
      const minPreviewWidth = 260; // Base min width when open
      const minMainWidth = 320;

      if (isResizing === 'left') {
        let newLeft = e.clientX - containerRect.left;
        if (newLeft < minCategoryWidth) newLeft = minCategoryWidth;
        // Don't let it squish main content too much
        const rightWidth = isPreviewOpen ? widthsRef.current.right : 0;
        const maxLeft = containerRect.width - rightWidth - minMainWidth;
        if (newLeft > maxLeft) newLeft = maxLeft;

        setWidths((prev) => ({ ...prev, left: newLeft }));
      } else {
        // Dragging left edge of right panel
        let newRight = containerRect.right - e.clientX;
        if (newRight < minPreviewWidth) newRight = minPreviewWidth;

        const maxRight = containerRect.width - widthsRef.current.left - minMainWidth;
        if (newRight > maxRight) newRight = maxRight;

        setWidths((prev) => ({ ...prev, right: newRight }));
      }
    };

    if (isResizing) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
    }
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing, setPanelWidths, isPreviewOpen]);

  // Mobile View: Stack Navigation
  if (isMobile) {
    return (
      <div className="w-full h-full relative overflow-hidden bg-base-100">
        <div
          className={`absolute inset-0 transition-transform duration-300 ${mobileActivePane === 'sidebar' ? 'translate-x-0' : '-translate-x-full'}`}
        >
          {leftPanel}
        </div>
        <div
          className={`absolute inset-0 transition-transform duration-300 ${mobileActivePane === 'grid' ? 'translate-x-0' : mobileActivePane === 'sidebar' ? 'translate-x-full' : '-translate-x-full'}`}
        >
          {mainPanel}
        </div>
        <div
          className={`absolute inset-0 transition-transform duration-300 ${mobileActivePane === 'details' ? 'translate-x-0' : 'translate-x-full'}`}
        >
          {rightPanel}
        </div>
      </div>
    );
  }

  // Desktop View: 3-Pane Split
  return (
    <div className="flex w-full h-full overflow-hidden" ref={containerRef}>
      {/* Left Panel */}
      <div
        style={{ width: widths.left }}
        className="h-full shrink-0 relative transition-[width] duration-0 ease-linear"
      >
        {leftPanel}
        <div
          className={`absolute top-0 right-0 w-1 h-full cursor-col-resize hover:bg-primary/50 transition-colors z-10 hidden md:flex items-center justify-center group ${isResizing === 'left' ? 'bg-primary' : 'bg-transparent'}`}
          onMouseDown={() => handleMouseDown('left')}
        >
          <div className="h-8 w-1 bg-base-content/10 rounded-full group-hover:h-full transition-all duration-300" />
        </div>
      </div>

      {/* Main Panel */}
      <div className="flex-1 h-full min-w-0 bg-transparent z-0 relative">{mainPanel}</div>

      {/* Right Panel */}
      <div
        style={{ width: isPreviewOpen ? widths.right : 0 }}
        className={`h-full shrink-0 relative transition-[width] duration-300 ease-in-out ${!isPreviewOpen ? 'overflow-hidden border-none' : ''}`}
      >
        {/* Resize Handle only if open */}
        {isPreviewOpen && (
          <div
            className={`absolute top-0 left-0 w-1 h-full cursor-col-resize hover:bg-primary/50 transition-colors z-10 hidden md:flex items-center justify-center group -ml-0.5 ${isResizing === 'right' ? 'bg-primary' : 'bg-transparent'}`}
            onMouseDown={() => handleMouseDown('right')}
          >
            <div className="h-8 w-1 bg-base-content/10 rounded-full group-hover:h-full transition-all duration-300" />
          </div>
        )}
        {rightPanel}
      </div>
    </div>
  );
}
