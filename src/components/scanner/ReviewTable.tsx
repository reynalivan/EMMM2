import { flexRender } from '@tanstack/react-table';
import type { ScanResultItem } from '../../types/scanner';
import { useReviewTable } from './useReviewTable';

interface Props {
  data: ScanResultItem[];
  onOpenFolder: (path: string) => void;
  onRename: (path: string, newName: string) => void;
}

export default function ReviewTable({ data, onOpenFolder, onRename }: Props) {
  // logic extracted to useReviewTable to handle React Compiler compatibility
  const table = useReviewTable({ data, onOpenFolder, onRename });

  return (
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
  );
}
