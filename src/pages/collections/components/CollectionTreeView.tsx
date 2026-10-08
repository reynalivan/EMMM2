import { useTranslation } from 'react-i18next';
import { ObjectRow } from './RecursiveCollectionTree';
import { VirtualCollectionTree } from './VirtualCollectionTree';
import { countTreeNodes, type CollectionTreeViewProps } from './collectionTreeStructure';
export type { CollectionTreeNodeChange } from './collectionTreeStructure';
const VIRTUAL_TREE_THRESHOLD = 80;
export function CollectionTreeView({
  nodes,
  gameId,
  colorClass = 'text-primary',
  emptyMessage,
  scrollElement,
  treeIdentity,
  nodeChanges,
}: CollectionTreeViewProps) {
  const { t } = useTranslation('collections');
  const tree = nodes ?? [];

  if (tree.length === 0) {
    return (
      <div className="rounded-xl border border-base-content/10 border-dashed bg-base-200/50 p-6 text-center text-sm text-base-content/40">
        {emptyMessage ?? t('preview.empty')}
      </div>
    );
  }

  if (countTreeNodes(tree) > VIRTUAL_TREE_THRESHOLD) {
    return (
      <VirtualCollectionTree
        nodes={tree}
        gameId={gameId}
        colorClass={colorClass}
        scrollElement={scrollElement}
        treeIdentity={treeIdentity}
        nodeChanges={nodeChanges}
      />
    );
  }

  return (
    <div className="space-y-1">
      {tree.map((objectNode) => (
        <ObjectRow
          key={objectNode.id}
          node={objectNode}
          colorClass={colorClass}
          gameId={gameId ?? ''}
          nodeChanges={nodeChanges}
        />
      ))}
    </div>
  );
}
