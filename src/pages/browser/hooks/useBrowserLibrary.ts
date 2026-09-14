import { type QueryClient, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { commands } from '@/shared/api/tauri/bindings';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';

const browserLibraryKeys = {
  all: ['browser-library'] as const,
  bookmarks: () => [...browserLibraryKeys.all, 'bookmarks'] as const,
  history: () => [...browserLibraryKeys.all, 'history'] as const,
  privacy: () => [...browserLibraryKeys.all, 'privacy'] as const,
};

async function refreshLibrary(queryClient: QueryClient): Promise<void> {
  await publishQueryScopes(queryClient, ['browserLibrary']);
}

export function useBrowserLibrary() {
  const queryClient = useQueryClient();
  const bookmarksQuery = useQuery({
    queryKey: browserLibraryKeys.bookmarks(),
    queryFn: () => commands.browserListBookmarks(),
    networkMode: 'always',
    staleTime: 30_000,
  });
  const historyQuery = useQuery({
    queryKey: browserLibraryKeys.history(),
    queryFn: () => commands.browserListHistory(100),
    networkMode: 'always',
    staleTime: 30_000,
  });
  const privacyQuery = useQuery({
    queryKey: browserLibraryKeys.privacy(),
    queryFn: () => commands.browserGetPrivacySummary(),
    networkMode: 'always',
    staleTime: 30_000,
  });

  const addBookmark = useMutation({
    mutationFn: ({ url, title, favicon }: { url: string; title: string; favicon: string | null }) =>
      commands.browserAddBookmark(url, title, favicon),
    networkMode: 'always',
    onSuccess: () => refreshLibrary(queryClient),
  });
  const deleteBookmark = useMutation({
    mutationFn: (id: string) => commands.browserDeleteBookmark(id),
    networkMode: 'always',
    onSuccess: () => refreshLibrary(queryClient),
  });
  const updateBookmark = useMutation({
    mutationFn: ({ id, url, title }: { id: string; url: string; title: string }) =>
      commands.browserUpdateBookmark(id, url, title),
    networkMode: 'always',
    onSuccess: () => refreshLibrary(queryClient),
  });
  const clearHistory = useMutation({
    mutationFn: () => commands.browserClearHistory(),
    networkMode: 'always',
    onSuccess: () => refreshLibrary(queryClient),
  });

  return {
    bookmarks: bookmarksQuery.data ?? [],
    history: historyQuery.data ?? [],
    privacySummary: privacyQuery.data ?? null,
    isLoading: bookmarksQuery.isLoading || historyQuery.isLoading,
    isRefreshing: bookmarksQuery.isRefetching || historyQuery.isRefetching,
    addBookmark,
    deleteBookmark,
    updateBookmark,
    clearHistory,
  };
}
