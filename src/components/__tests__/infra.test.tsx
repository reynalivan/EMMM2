import { render, screen } from '../../test-utils';
import { describe, it, expect } from 'vitest';

describe('Frontend Infrastructure', () => {
  it('renders with custom provider', () => {
    render(<div data-testid="test-div">Hello Vibecode</div>);
    expect(screen.getByTestId('test-div')).toBeInTheDocument();
  });

  it('can access global mocks', () => {
    expect(window.matchMedia).toBeDefined();
    expect(window.ResizeObserver).toBeDefined();
  });
});
