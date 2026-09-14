/** Duration of the splash fade-out; must match the transition in `index.html`. */
const SPLASH_FADE_MS = 160;

/** Remove the static boot splash once the selected React entry point is ready. */
export function dismissSplash() {
  const splash = document.getElementById('splash');
  if (!splash) return;

  splash.classList.add('is-done');
  window.setTimeout(() => splash.remove(), SPLASH_FADE_MS);
}
