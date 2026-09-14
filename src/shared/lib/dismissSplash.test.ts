import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { dismissSplash } from './dismissSplash';

describe('dismissSplash', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = '<div id="splash"></div>';
  });

  afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = '';
  });

  it('fades the boot surface before removing it', () => {
    dismissSplash();

    const splash = document.getElementById('splash');
    expect(splash).toHaveClass('is-done');

    vi.advanceTimersByTime(159);
    expect(document.getElementById('splash')).not.toBeNull();

    vi.advanceTimersByTime(1);
    expect(document.getElementById('splash')).toBeNull();
  });
});
