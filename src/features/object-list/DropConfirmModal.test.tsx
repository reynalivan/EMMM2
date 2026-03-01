import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeAll } from 'vitest';
import DropConfirmModal, { type DropValidation } from './DropConfirmModal';

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = vi.fn();
  HTMLDialogElement.prototype.close = vi.fn();
});

describe('DropConfirmModal', () => {
  it('renders nothing when validation is null', () => {
    const { container } = render(
      <DropConfirmModal
        validation={null}
        onMoveAnyway={vi.fn()}
        onMoveToSuggested={vi.fn()}
        onCancel={vi.fn()}
        onSkipValidation={vi.fn()}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('shows validating state correctly', () => {
    const validation: DropValidation = {
      paths: ['f1', 'f2'],
      targetId: 't1',
      targetName: 'Target 1',
      status: 'validating',
    };
    const onSkipValidation = vi.fn();
    render(
      <DropConfirmModal
        validation={validation}
        onMoveAnyway={vi.fn()}
        onMoveToSuggested={vi.fn()}
        onCancel={vi.fn()}
        onSkipValidation={onSkipValidation}
      />,
    );

    expect(screen.getByText(/Checking match confidence/i)).toBeInTheDocument();
    fireEvent.click(screen.getByText('Skip Validation'));
    expect(onSkipValidation).toHaveBeenCalled();
  });

  it('shows warning state with suggestion', () => {
    const validation: DropValidation = {
      paths: ['f1'],
      targetId: 't1',
      targetName: 'Target 1',
      status: 'warning',
      targetScore: 20,
      suggestedId: 't2',
      suggestedName: 'Target 2',
      suggestedScore: 90,
    };
    const onMoveToSuggested = vi.fn();
    render(
      <DropConfirmModal
        validation={validation}
        onMoveAnyway={vi.fn()}
        onMoveToSuggested={onMoveToSuggested}
        onCancel={vi.fn()}
        onSkipValidation={vi.fn()}
      />,
    );

    expect(screen.getByText('Low Match Confidence')).toBeInTheDocument();
    expect(screen.getByText('Target 2')).toBeInTheDocument();

    fireEvent.click(screen.getByText(/Move to Target 2/i));
    expect(onMoveToSuggested).toHaveBeenCalled();
  });
});
