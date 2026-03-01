import { render, screen } from '@testing-library/react';
import { ContextMenu } from '../../components/ui/ContextMenu';
import { describe, it, expect, vi } from 'vitest';

// Mock Lucide icons
vi.mock('lucide-react', () => ({
  ExternalLink: () => <div data-testid="icon-external-link" />,
  // Add others if needed
}));

describe('ContextMenu', () => {
  it('renders children', () => {
    render(
      <ContextMenu content={<div>Menu Content</div>}>
        <div data-testid="trigger">Right Click Me</div>
      </ContextMenu>,
    );
    expect(screen.getByTestId('trigger')).toBeInTheDocument();
  });

  // Since ContextMenu uses Radix UI, testing interaction requires userEvent or pointer events
  // For basic unit test, verifying render is enough.
});
