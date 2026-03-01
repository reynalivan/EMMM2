import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import DragOverlay from './DragOverlay';

describe('DragOverlay', () => {
  it('renders correctly when dragging', () => {
    render(<DragOverlay isDragging={true} />);
    expect(screen.getByText('Drop files here')).toBeInTheDocument();
  });

  it('renders nothing when not dragging', () => {
    const { container } = render(<DragOverlay isDragging={false} />);
    expect(container.firstChild).toBeNull();
  });
});
