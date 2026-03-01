import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ObjectListContent from './ObjectListContent';
import type { FlatItem } from './useObjectListVirtualizer';

vi.mock('../../components/ui/ContextMenu', () => ({
  ContextMenu: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
vi.mock('./ObjectRowItem', () => ({
  default: ({ obj }: { obj: { name: string } }) => <div data-testid="row-item">{obj.name}</div>,
}));
vi.mock('./CategorySection', () => ({
  default: ({ category }: { category: { name: string } }) => (
    <div data-testid="category-section">{category.name}</div>
  ),
}));
vi.mock('./ObjectContextMenu', () => ({
  ObjectContextMenu: () => <div />,
}));

describe('ObjectListContent', () => {
  it('renders virtualized list correctly', () => {
    const mockVirtualizerFactory = () => {
      const totalSize = 100;
      const virtualItems = [
        { index: 0, size: 50, start: 0 },
        { index: 1, size: 50, start: 50 },
      ];
      return {
        getTotalSize: () => totalSize,
        getVirtualItems: () => virtualItems,
      };
    };

    const flatItems: FlatItem[] = [
      {
        type: 'header',
        category: { name: 'Chars' } as unknown as React.ComponentProps<
          typeof ObjectListContent
        >['flatObjectItems'][0] extends { type: 'header' }
          ? React.ComponentProps<typeof ObjectListContent>['flatObjectItems'][0]['category']
          : never,
        count: 1,
      },
      {
        type: 'row',
        obj: { id: '1', name: 'Obj1' } as unknown as React.ComponentProps<
          typeof ObjectListContent
        >['flatObjectItems'][0] extends { type: 'row' }
          ? React.ComponentProps<typeof ObjectListContent>['flatObjectItems'][0]['obj']
          : never,
      },
    ];

    render(
      <ObjectListContent
        parentRef={{ current: null }}
        rowVirtualizer={
          mockVirtualizerFactory() as unknown as import('@tanstack/react-virtual').Virtualizer<
            HTMLDivElement,
            Element
          >
        }
        flatObjectItems={flatItems}
        selectedObject={null}
        selectedObjectType={null}
        setSelectedObject={vi.fn()}
        setSelectedObjectType={vi.fn()}
        isMobile={false}
        stickyPosition={null}
        selectedIndex={-1}
        scrollToSelected={vi.fn()}
        contextMenuProps={
          {} as unknown as React.ComponentProps<typeof ObjectListContent>['contextMenuProps']
        }
      />,
    );

    expect(screen.getByTestId('category-section')).toBeInTheDocument();
    expect(screen.getByTestId('row-item')).toBeInTheDocument();
  });
});
