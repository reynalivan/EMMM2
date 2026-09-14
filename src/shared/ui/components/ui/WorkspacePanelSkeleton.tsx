interface WorkspacePanelSkeletonProps {
  variant: 'grid' | 'list' | 'preview' | 'inbox';
}

export default function WorkspacePanelSkeleton({ variant }: WorkspacePanelSkeletonProps) {
  if (variant === 'preview') {
    return (
      <div className="w-full space-y-5 p-6" aria-hidden="true">
        <div className="space-y-2">
          <div className="skeleton h-4 w-2/5 bg-base-300/70" />
          <div className="skeleton h-3 w-3/5 bg-base-300/55" />
        </div>
        <div className="skeleton aspect-video w-full rounded-xl bg-base-300/70" />
        <div className="space-y-2">
          <div className="skeleton h-3 w-1/4 bg-base-300/70" />
          <div className="skeleton h-3 w-full bg-base-300/55" />
          <div className="skeleton h-3 w-4/5 bg-base-300/55" />
        </div>
      </div>
    );
  }

  if (variant === 'inbox') {
    return (
      <div className="mx-auto w-full max-w-6xl space-y-3" aria-hidden="true">
        {Array.from({ length: 5 }, (_, index) => (
          <div key={index} className="workspace-surface flex items-center gap-4 p-4">
            <div className="skeleton h-5 w-5 rounded bg-base-300/70" />
            <div className="skeleton h-11 w-11 rounded-lg bg-base-300/70" />
            <div className="min-w-0 flex-1 space-y-2">
              <div className="skeleton h-3 w-2/5 bg-base-300/70" />
              <div className="skeleton h-2.5 w-4/5 bg-base-300/55" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  if (variant === 'list') {
    return (
      <div className="w-full space-y-3 px-3 py-4" aria-hidden="true">
        {Array.from({ length: 6 }, (_, index) => (
          <div key={index} className="flex items-center gap-3 rounded-lg px-2 py-1.5">
            <div className="skeleton h-14 w-14 shrink-0 rounded-xl bg-base-300/70" />
            <div className="min-w-0 flex-1 space-y-2">
              <div className="skeleton h-3 w-2/5 bg-base-300/70" />
              <div className="skeleton h-2.5 w-1/4 bg-base-300/55" />
            </div>
            <div className="skeleton h-5 w-9 rounded-full bg-base-300/55" />
          </div>
        ))}
      </div>
    );
  }

  return (
    <div
      className="grid w-full grid-cols-2 gap-3 px-4 py-4 sm:grid-cols-3 xl:grid-cols-4"
      aria-hidden="true"
    >
      {Array.from({ length: 8 }, (_, index) => (
        <div key={index} className="overflow-hidden rounded-lg border border-base-content/5">
          <div className="skeleton aspect-square bg-base-300/70" />
          <div className="space-y-2 bg-base-200/55 p-3">
            <div className="skeleton h-3 w-3/5 bg-base-300/70" />
            <div className="skeleton h-2.5 w-2/5 bg-base-300/55" />
          </div>
        </div>
      ))}
    </div>
  );
}
