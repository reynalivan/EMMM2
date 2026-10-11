import { QueryClient } from '@tanstack/react-query';
import { expect, it } from 'vitest';
import { refreshCatalogQueries } from './catalogQueries';

it('invalidates infinite catalog caches and derived lookups without touching unrelated data', async () => {
  const client = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } });
  const affected = [
    ['catalog-pack', 'status'],
    ['master-db', 0],
    ['search-master-db', 0, 'Amber'],
    ['suggest-master-db', 0, 'Amber'],
    ['object-identity-suggestions', 'game'],
    ['thumbnails', 'folder'],
  ];
  for (const key of [...affected, ['browser-history']]) client.setQueryData(key, []);
  await refreshCatalogQueries(client);
  for (const key of affected) expect(client.getQueryState(key)?.isInvalidated).toBe(true);
  expect(client.getQueryState(['browser-history'])?.isInvalidated).toBe(false);
  client.clear();
});

it('cancels an old in-flight read so a late empty response cannot replace new catalog data', async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  let finish!: (value: string[]) => void;
  const oldRead = client
    .fetchQuery({
      queryKey: ['master-db', 0],
      queryFn: () =>
        new Promise<string[]>((resolve) => {
          finish = resolve;
        }),
    })
    .catch(() => undefined);
  await refreshCatalogQueries(client);
  client.setQueryData(['master-db', 0], ['Amber']);
  finish([]);
  await oldRead;
  expect(client.getQueryData(['master-db', 0])).toEqual(['Amber']);
  client.clear();
});
