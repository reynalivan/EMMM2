/**
 * Tests for US-07.4: Drag-and-Drop Mod Re-Categorization
 * Covers:
 * - TC-07-10: Seamless Drag and Drop Flow (Confirming Move)
 * - TC-07-11: DnD mapping collision/warnings (Validating/Warning States)
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeAll } from 'vitest';
import DropConfirmModal, { type DropValidation } from './DropConfirmModal';

beforeAll(() => {
  if (!HTMLDialogElement.prototype.showModal) {
    HTMLDialogElement.prototype.showModal = function () {
      this.open = true;
    };
  }
  if (!HTMLDialogElement.prototype.close) {
    HTMLDialogElement.prototype.close = function () {
      this.open = false;
    };
  }
});

describe('DropConfirmModal (Drag and Drop Validation)', () => {
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
    expect(container).toBeEmptyDOMElement();
  });

  it('renders validating state correctly', () => {
    const validation: DropValidation = {
      paths: ['C:\\some\\path'],
      targetId: 'obj-1',
      targetName: 'Diluc',
      status: 'validating',
    };

    render(
      <DropConfirmModal
        validation={validation}
        onMoveAnyway={vi.fn()}
        onMoveToSuggested={vi.fn()}
        onCancel={vi.fn()}
        onSkipValidation={vi.fn()}
      />,
    );

    expect(screen.getByText(/checking match confidence/i)).toBeInTheDocument();
  });

  it('renders warning state for low confidence match (TC-07-11)', () => {
    const validation: DropValidation = {
      paths: ['C:\\some\\path'],
      targetId: 'obj-1',
      targetName: 'Diluc',
      status: 'warning',
      targetScore: 30,
    };

    render(
      <DropConfirmModal
        validation={validation}
        onMoveAnyway={vi.fn()}
        onMoveToSuggested={vi.fn()}
        onCancel={vi.fn()}
        onSkipValidation={vi.fn()}
      />,
    );

    expect(screen.getByText(/low match confidence/i)).toBeInTheDocument();
    expect(screen.getByText(/30% match/i)).toBeInTheDocument();

    // Suggestion box should not be visible since there's no suggestion
    expect(screen.queryByText(/suggested target/i)).not.toBeInTheDocument();
  });

  it('renders suggested target when available (TC-07-10)', () => {
    const validation: DropValidation = {
      paths: ['C:\\some\\path'],
      targetId: 'obj-1',
      targetName: 'Diluc',
      status: 'warning',
      targetScore: 30,
      suggestedId: 'obj-2',
      suggestedName: 'Hu Tao',
      suggestedScore: 90,
    };

    const mockMoveSuggested = vi.fn();

    render(
      <DropConfirmModal
        validation={validation}
        onMoveAnyway={vi.fn()}
        onMoveToSuggested={mockMoveSuggested}
        onCancel={vi.fn()}
        onSkipValidation={vi.fn()}
      />,
    );

    expect(screen.getByText(/suggested target:/i)).toBeInTheDocument();
    expect(screen.getByText('Hu Tao')).toBeInTheDocument();

    const moveBtn = screen.getByRole('button', { name: /move to hu tao/i });
    fireEvent.click(moveBtn);
    expect(mockMoveSuggested).toHaveBeenCalled();
  });

  it('calls onMoveAnyway when manually triggered despite warning', () => {
    const validation: DropValidation = {
      paths: ['C:\\some\\path'],
      targetId: 'obj-1',
      targetName: 'Diluc',
      status: 'warning',
      targetScore: 10,
    };

    const mockMoveAnyway = vi.fn();

    render(
      <DropConfirmModal
        validation={validation}
        onMoveAnyway={mockMoveAnyway}
        onMoveToSuggested={vi.fn()}
        onCancel={vi.fn()}
        onSkipValidation={vi.fn()}
      />,
    );

    const moveBtn = screen.getByRole('button', { name: /move anyway/i });
    fireEvent.click(moveBtn);
    expect(mockMoveAnyway).toHaveBeenCalled();
  });
});
