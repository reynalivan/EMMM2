import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import CategorySection from './CategorySection';

describe('CategorySection', () => {
  const dummyCategory = { name: 'Character', label: 'Characters', icon: 'User', color: 'primary' };

  it('renders correctly and handles click', () => {
    const onSelect = vi.fn();
    render(
      <CategorySection category={dummyCategory} count={5} isSelected={false} onSelect={onSelect} />,
    );

    expect(screen.getByText('Characters')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Characters'));
    expect(onSelect).toHaveBeenCalled();
  });
});
