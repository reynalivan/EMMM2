import { render, screen } from '../../testing/test-utils';
import { describe, it, expect } from 'vitest';

describe('Frontend Infrastructure', () => {
  it('renders with custom provider', () => {
    render(<div data-testid="test-div">Hello EMMM2</div>);
    expect(screen.getByTestId('test-div')).toBeInTheDocument();
  });

  it('can access global mocks', () => {
    expect(window.matchMedia).toBeDefined();
    expect(window.ResizeObserver).toBeDefined();
  });
});
