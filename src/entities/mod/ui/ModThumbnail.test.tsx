import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '../../../tests/testing/test-utils';
import { ModThumbnail } from './ModThumbnail';

const mockUseThumbnail = vi.fn();

vi.mock('../api/useThumbnail', () => ({
  useThumbnail: (...args: unknown[]) => mockUseThumbnail(...args),
}));

describe('ModThumbnail', () => {
  it('uses the shared cached thumbnail query when no source URL is supplied', () => {
    mockUseThumbnail.mockReturnValue({ data: 'asset://cached-thumb', isLoading: false });

    const { container } = render(
      <ModThumbnail gameId="g-1" folderPath="Character/Mod A" sizeClassName="size-9" />,
    );

    expect(mockUseThumbnail).toHaveBeenCalledWith('g-1', 'Character/Mod A', true);
    expect(container.querySelector('img')?.getAttribute('src')).toBe('asset://cached-thumb');
  });

  it('uses an already-resolved thumbnail URL without issuing another query', () => {
    mockUseThumbnail.mockReturnValue({ data: null, isLoading: false });

    const { container } = render(
      <ModThumbnail gameId="g-1" thumbnailSrc="asset://randomizer-thumb" sizeClassName="size-10" />,
    );

    expect(mockUseThumbnail).toHaveBeenCalledWith('g-1', '', false);
    expect(container.querySelector('img')?.getAttribute('src')).toBe('asset://randomizer-thumb');
  });

  it('renders the fallback when the thumbnail system has no image', () => {
    mockUseThumbnail.mockReturnValue({ data: null, isLoading: false });

    const { container } = render(
      <ModThumbnail gameId="g-1" folderPath="Character/No Preview" sizeClassName="size-9" />,
    );

    expect(container.querySelector('img')).toBeNull();
    expect(screen.getByTestId('icon-imageoff')).toBeInTheDocument();
  });
});
