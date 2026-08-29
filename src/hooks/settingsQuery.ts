import { commands } from '../lib/bindings';

export const settingsKeys = {
  all: ['settings'] as const,
};

export const settingsQueryOptions = {
  queryKey: settingsKeys.all,
  queryFn: () => commands.getSettings(),
  staleTime: Infinity, // Settings don't change often from outside
};
