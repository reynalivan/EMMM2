/**
 * buildModTree — Derives a recursive Object > [Dir] > Mod hierarchy
 * from a flat CollectionMember list.
 */

import {
  CollectionMember,
  CollectionMod,
  CollectionRoot,
} from '../../../types/collection';

// ─── Tree Node Types ──────────────────────────────────────────────────────────

export interface ModTreeNode {
  kind: 'mod';
  id: string;
  name: string;
  mod_path: string;
  is_enabled: boolean;
  effectively_disabled: boolean;
}

export interface DirTreeNode {
  kind: 'dir';
  id: string;
  name: string;
  path: string;
  is_enabled: boolean;
  children: (DirTreeNode | ModTreeNode)[];
}

export interface ObjectTreeNode {
  kind: 'object';
  id: string;
  name: string;
  is_enabled: boolean;
  children: (DirTreeNode | ModTreeNode)[];
  modCount: number;
}

// ─── Builder ──────────────────────────────────────────────────────────────────

export function buildModTree(members: CollectionMember[]): ObjectTreeNode[] {
  const objectMap = new Map<string, ObjectTreeNode>();
  const allRoots: CollectionRoot[] = [];
  const allMods: CollectionMod[] = [];
  const UNCATEGORIZED_ID = '__uncategorized__';

  // 1. Harvest everything and create Object placeholders
  for (const m of members) {
    if (m.kind === 'object') {
      const objId = m.object_id || m.path_key || 'unknown';
      if (!objectMap.has(objId)) {
        objectMap.set(objId, {
          kind: 'object',
          id: objId,
          name: m.display_name || 'Unknown',
          is_enabled: m.is_enabled,
          children: [],
          modCount: 0,
        });
      }
    } else if (m.kind === 'root') {
      allRoots.push(m);
    } else if (m.kind === 'mod' || m.kind === 'nested') {
      allMods.push(m);
    }
  }

  // 2. Heal Mods (Path-based matching)
  // For each mod, we find the longest matching root path to determine its "True" Object ID.
  // This takes precedence over m.object_id if a root matches.
  const modsByObject = new Map<string, CollectionMod[]>();

  for (const m of allMods) {
    const normModPath = normalizePathSep(m.mod_path).toLowerCase();
    let bestRoot: CollectionRoot | null = null;
    let longestMatch = -1;

    for (const r of allRoots) {
      const rPath = normalizePathSep(r.root_path).toLowerCase();
      if (normModPath.startsWith(rPath) && rPath.length > longestMatch) {
         // Valid match if exact or followed by separator
         if (normModPath.length === rPath.length || normModPath[rPath.length] === '/') {
           bestRoot = r;
           longestMatch = rPath.length;
         }
      }
    }

    const finalObjId = bestRoot?.object_id || m.object_id || UNCATEGORIZED_ID;
    const list = modsByObject.get(finalObjId) || [];
    list.push(m);
    modsByObject.set(finalObjId, list);

    // If finalObjId isn't in objectMap, create it using best info
    if (!objectMap.has(finalObjId)) {
      let name = finalObjId === UNCATEGORIZED_ID ? 'Uncategorized' : finalObjId;
      if (bestRoot?.display_name) name = bestRoot.display_name;

      objectMap.set(finalObjId, {
        kind: 'object',
        id: finalObjId,
        name,
        is_enabled: true,
        children: [],
        modCount: 0,
      });
    }
    objectMap.get(finalObjId)!.modCount++;
  }

  // 3. Build subtrees
  for (const [objId, obj] of objectMap.entries()) {
    const mods = modsByObject.get(objId) || [];
    const objRoots = allRoots.filter(r => r.object_id === objId);

    if (mods.length === 0) continue;

    // ROOT FLATTENING:
    // If we have exactly 1 root and it matches the object name/id, 
    // or we have 0 roots, group mods directly under the object.
    const shouldFlatten = objRoots.length <= 1;

    if (shouldFlatten) {
      const basePath = objRoots[0]?.root_path || '';
      const parentEnabled = obj.is_enabled && (objRoots[0]?.is_enabled ?? true);
      buildRecursiveSubtree(obj.children, mods, basePath, parentEnabled);
    } else {
      // Multiple roots: Create folders for each root
      for (const r of objRoots) {
        const rootPath = normalizePathSep(r.root_path).toLowerCase();
        const rootMods = mods.filter(m => normalizePathSep(m.mod_path).toLowerCase().startsWith(rootPath));
        
        const rootDir: DirTreeNode = {
          kind: 'dir',
          id: r.root_path_key || r.root_path,
          name: r.display_name || r.root_path.split(/[/\\]/).pop() || 'Root',
          path: r.root_path,
          is_enabled: r.is_enabled,
          children: [],
        };
        
        buildRecursiveSubtree(rootDir.children, rootMods, r.root_path, obj.is_enabled && r.is_enabled);
        if (rootDir.children.length > 0) {
          obj.children.push(rootDir);
        }
      }
      
      // Edge case: Mods for this object that didn't match any of its specific roots
      const orphans = mods.filter(m => {
        const p = normalizePathSep(m.mod_path).toLowerCase();
        return !objRoots.some(r => p.startsWith(normalizePathSep(r.root_path).toLowerCase()));
      });
      if (orphans.length > 0) {
        buildRecursiveSubtree(obj.children, orphans, '', obj.is_enabled);
      }
    }

    // Sort children: Dirs first, then name
    obj.children.sort((a, b) => {
      if (a.kind === 'dir' && b.kind === 'mod') return -1;
      if (a.kind === 'mod' && b.kind === 'dir') return 1;
      return a.name.localeCompare(b.name, undefined, { numeric: true });
    });
  }

  // Final Object sorting
  return Array.from(objectMap.values())
    .filter(obj => obj.modCount > 0 || obj.id !== UNCATEGORIZED_ID)
    .sort((a, b) => {
      if (a.id === UNCATEGORIZED_ID) return 1;
      if (b.id === UNCATEGORIZED_ID) return -1;
      return a.name.localeCompare(b.name);
    });
}

function buildRecursiveSubtree(
  target: (DirTreeNode | ModTreeNode)[],
  mods: CollectionMod[],
  basePath: string,
  parentEnabled: boolean,
) {
  const normBase = normalizePathSep(basePath).toLowerCase();

  for (const m of mods) {
    const normModPath = normalizePathSep(m.mod_path);
    const normModPathLower = normModPath.toLowerCase();

    // Skip if mod is not under this base path
    if (normBase && !normModPathLower.startsWith(normBase)) continue;

    // Relative path calculation using normalized forms
    let rel = normBase ? normModPath.substring(normBase.length) : normModPath;
    if (rel.startsWith('/') || rel.startsWith('\\')) rel = rel.substring(1);

    const segments = rel.split(/[/\\]/).filter(Boolean);
    const modFileName = segments.pop() || m.display_name || 'mod.ini';

    let currentLevel = target;
    let currentPath = basePath;

    for (const segment of segments) {
      currentPath = currentPath ? `${currentPath}/${segment}` : segment;
      let dir = currentLevel.find(
        (n) => n.kind === 'dir' && n.name.toLowerCase() === segment.toLowerCase(),
      ) as DirTreeNode | undefined;
      if (!dir) {
        dir = {
          kind: 'dir',
          id: currentPath,
          name: segment,
          path: currentPath,
          is_enabled: true,
          children: [],
        };
        currentLevel.push(dir);
      }
      currentLevel = dir.children;
    }

    currentLevel.push({
      kind: 'mod',
      id: m.mod_path_key ?? m.mod_path,
      name: m.display_name || modFileName,
      mod_path: m.mod_path,
      is_enabled: m.is_enabled,
      effectively_disabled: !parentEnabled || !m.is_enabled,
    });
  }
}

function normalizePathSep(p: string): string {
  if (!p) return '';
  return p.replace(/\\/g, '/');
}
