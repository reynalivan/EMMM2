import { useState, useEffect } from 'react';

const MOBILE_QUERY = '(max-width: 767px)';

/**
 * Custom hook to track responsive state.
 * @returns {boolean} isMobile - True if the viewport is under 768px wide
 */
export function useResponsive() {
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const media = window.matchMedia(MOBILE_QUERY);
    const sync = () => setIsMobile(media.matches);

    sync();

    media.addEventListener('change', sync);
    return () => media.removeEventListener('change', sync);
  }, []);

  return { isMobile };
}
