import { flexRender } from '@tanstack/react-table';
import type { ScanResultItem } from '../../types/scanner';
import { useReviewTable } from './useReviewTable';

interface Props {
  data: ScanResultItem[];
  onOpenFolder: (path: string) => void;
  onRename: (path: string, newName: string) => void;
  onBulkEnable?: (paths: string[]) => void;
  onBulkDisable?: (paths: string[]) => void;
  onBulkDelete?: (paths: string[]) => void;
  onAutoOrganize?: (paths: string[]) => void;
}

export default function ReviewTable({
  data,
  onOpenFolder,
  onRename,
  onBulkEnable,
  onBulkDisable,
  onBulkDelete,
  onAutoOrganize,
}: Props) {
  // logic extracted to useReviewTable to handle React Compiler compatibility
  const table = useReviewTable({ data, onOpenFolder, onRename });

  const selectedRows = table.getSelectedRowModel().rows;
  const selectedPaths = selectedRows.map((r) => r.original.path);

  return (
    <div className="flex flex-col gap-2">
      {/* Bulk Actions Toolbar */}
      {selectedPaths.length > 0 && (
        <div className="flex items-center gap-2 p-2 bg-base-200 rounded-lg animate-in fade-in slide-in-from-top-1">
          <span className="text-sm font-medium px-2">{selectedPaths.length} selected</span>
          <div className="divider divider-horizontal my-0"></div>
          {onBulkEnable && (
            <button
              className="btn btn-xs btn-success btn-outline"
              onClick={() => {
                onBulkEnable(selectedPaths);
                table.resetRowSelection();
              }}
            >
              Enable
            </button>
          )}
          {onBulkDisable && (
            <button
              className="btn btn-xs btn-error btn-outline"
              onClick={() => {
                onBulkDisable(selectedPaths);
                table.resetRowSelection();
              }}
            >
              Disable
            </button>
          )}
          {onAutoOrganize && (
            <button
              className="btn btn-xs btn-info btn-outline"
              onClick={() => {
                onAutoOrganize(selectedPaths);
                table.resetRowSelection();
              }}
            >
              Auto Organize
            </button>
          )}
          {onBulkDelete && (
            <button
              className="btn btn-xs btn-ghost text-error"
              onClick={() => {
                onBulkDelete(selectedPaths);
                table.resetRowSelection();
              }}
            >
              Delete
            </button>
          )}
        </div>
      )}

      <div className="overflow-x-auto border border-base-300 rounded-lg bg-base-100">
        <table className="table table-sm table-pin-rows">
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="bg-base-200 text-base-content/70">
                {headerGroup.headers.map((header) => (
                  <th key={header.id}>
                    {header.isPlaceholder
                      ? null
                      : flexRender(header.column.columnDef.header, header.getContext())}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.id} className="hover:bg-base-200/50">
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id}>{flexRender(cell.column.columnDef.cell, cell.getContext())}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>

        {data.length === 0 && (
          <div className="text-center py-8 text-base-content/50 text-sm">
            No mods found. Try scanning again.
          </div>
        )}
      </div>
    </div>
  );
}
