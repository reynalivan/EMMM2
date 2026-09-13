import { createRef, type ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '../../../tests/testing/test-utils';
import ObjectListDropIndicators from './ObjectListDropIndicators';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

describe('ObjectListDropIndicators', () => {
  it('floats the object count above the list without reserving footer space', () => {
    render(
      <ObjectListDropIndicators
        isDragging={false}
        activeDropZone={null}
        hoveredItemId={null}
        tooltipTop={0}
        objects={[]}
        selectedObjectType={null}
        objectCount={6}
        onShowAll={vi.fn()}
        bottomRef={createRef<HTMLDivElement>()}
      />,
    );

    expect(screen.getByText('6 objects')).toBeInTheDocument();
    expect(screen.getByTestId('object-list-count-overlay')).toHaveClass(
      'absolute',
      'bottom-0',
      'pointer-events-none',
    );
  });
});
