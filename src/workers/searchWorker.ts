/**
 * US-3.6: Search Web Worker — instant sidebar search offloaded from main thread.
 * Receives object data and query, returns matching IDs using multi-token substring matching.
 */

interface SearchItem {
  id: string;
  name: string;
}

interface SearchMessage {
  objects: SearchItem[];
  query: string;
}

self.onmessage = (e: MessageEvent<SearchMessage>) => {
  const { objects, query } = e.data;

  if (!query || query.trim().length === 0) {
    self.postMessage({ ids: null });
    return;
  }

  const tokens = query.toLowerCase().trim().split(/\s+/);

  const matchedIds = objects
    .filter((obj) => {
      const name = obj.name.toLowerCase();
      return tokens.every((t) => name.includes(t));
    })
    .map((obj) => obj.id);

  self.postMessage({ ids: matchedIds });
};

export {};
