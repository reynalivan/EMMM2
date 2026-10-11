export type BrowserNavigationUpdate = {
  url: string;
  title?: string;
};

export function retainPendingBrowserNavigation(
  pending: Map<string, BrowserNavigationUpdate>,
  label: string,
  update: BrowserNavigationUpdate,
): void {
  const previous = pending.get(label);
  pending.set(label, {
    ...previous,
    ...update,
    title: update.title ?? previous?.title,
  });
}

export function takePendingBrowserNavigation(
  pending: Map<string, BrowserNavigationUpdate>,
  label: string,
): BrowserNavigationUpdate | undefined {
  const update = pending.get(label);
  pending.delete(label);
  return update;
}
