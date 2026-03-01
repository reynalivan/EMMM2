/* eslint-disable react-hooks/incompatible-library */
import { useMemo, useState } from 'react';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  createColumnHelper,
  SortingState,
  Table,
} from '@tanstack/react-table';
import { convertFileSrc } from '@tauri-apps/api/core';
import { FolderOpen, ArrowUpDown, Sparkles } from 'lucide-react';
import { ScanResultItem } from '../../../types/scanner';
import { IndeterminateCheckbox, EditableCell } from './ReviewTableComponents';

const columnHelper = createColumnHelper<ScanResultItem>();

interface UseReviewTableProps {
  data: ScanResultItem[];
  onOpenFolder: (path: string) => void;
  onRename: (path: string, newName: string) => void;
}

export function useReviewTable({
  data,
  onOpenFolder,
  onRename,
}: UseReviewTableProps): Table<ScanResultItem> {
  'use no memo'; // Opt out: useReactTable returns un-memoizable functions

  const [sorting, setSorting] = useState<SortingState>([]);
  const [rowSelection, setRowSelection] = useState({});

  const columns = useMemo(
    () => [
      // Selection
      columnHelper.display({
        id: 'select',
        header: ({ table }) => (
          <label>
            <IndeterminateCheckbox
              checked={table.getIsAllRowsSelected()}
              isIndeterminate={table.getIsSomeRowsSelected()}
              onChange={table.getToggleAllRowsSelectedHandler()}
              className="checkbox checkbox-xs"
            />
          </label>
        ),
        cell: ({ row }) => (
          <label>
            <IndeterminateCheckbox
              checked={row.getIsSelected()}
              disabled={!row.getCanSelect()}
              isIndeterminate={row.getIsSomeSelected()}
              onChange={row.getToggleSelectedHandler()}
              className="checkbox checkbox-xs"
            />
          </label>
        ),
      }),
      // Thumbnail
      columnHelper.accessor('thumbnailPath', {
        id: 'thumbnail',
        header: 'Preview',
        cell: (info) => {
          const path = info.getValue();
          if (!path) return <div className="w-10 h-10 bg-base-300 rounded-md" />;

          return (
            <div className="avatar">
              <div className="w-10 h-10 rounded-md">
                <img src={convertFileSrc(path)} alt="mod preview" className="object-cover" />
              </div>
            </div>
          );
        },
      }),
      // Name (Editable)
      columnHelper.accessor('displayName', {
        header: ({ column }) => (
          <button className="btn btn-ghost btn-xs gap-1" onClick={column.getToggleSortingHandler()}>
            Mod Name
            <ArrowUpDown className="w-3 h-3" />
          </button>
        ),
        cell: (info) => (
          <EditableCell
            value={info.getValue()}
            rawName={info.row.original.rawName}
            path={info.row.original.path}
            onRename={onRename}
          />
        ),
      }),
      // Matched Object
      columnHelper.accessor('matchedObject', {
        header: 'Category',
        cell: (info) => {
          const val = info.getValue();
          return val ? (
            <span className="badge badge-sm badge-outline">{val}</span>
          ) : (
            <span className="text-base-content/30 text-xs">-</span>
          );
        },
      }),
      // Detected Skin
      columnHelper.accessor('detectedSkin', {
        header: () => (
          <span className="flex items-center gap-1">
            <Sparkles className="w-3 h-3" />
            Skin
          </span>
        ),
        cell: (info) => {
          const skin = info.getValue();
          const folderName = info.row.original.skinFolderName;
          if (!skin) return <span className="text-base-content/20 text-xs">—</span>;
          return (
            <div className="flex flex-col gap-0.5">
              <span className="badge badge-xs badge-primary badge-outline truncate max-w-[140px]">
                {skin}
              </span>
              {folderName && (
                <span className="text-[10px] text-base-content/40 font-mono truncate max-w-[140px]">
                  → {folderName}
                </span>
              )}
            </div>
          );
        },
      }),
      // Confidence
      columnHelper.accessor('confidence', {
        header: 'Confidence',
        cell: (info) => {
          const val = info.getValue();
          const color =
            val === 'High' ? 'badge-success' : val === 'Medium' ? 'badge-warning' : 'badge-ghost';
          return <span className={`badge badge-xs ${color} badge-soft`}>{val}</span>;
        },
      }),
      // Status
      columnHelper.accessor('isDisabled', {
        header: 'Status',
        cell: (info) => (
          <span
            className={`badge badge-xs ${info.getValue() ? 'badge-error badge-outline' : 'badge-success badge-outline'}`}
          >
            {info.getValue() ? 'Disabled' : 'Enabled'}
          </span>
        ),
      }),
      // Actions
      columnHelper.display({
        id: 'actions',
        cell: (info) => (
          <div className="flex gap-2 justify-end">
            <button
              className="btn btn-ghost btn-xs btn-square tooltip tooltip-left"
              data-tip="Open Folder"
              onClick={() => onOpenFolder(info.row.original.path)}
            >
              <FolderOpen className="w-4 h-4" />
            </button>
          </div>
        ),
      }),
    ],
    [onOpenFolder, onRename],
  );

  const table = useReactTable({
    data,
    columns,
    state: {
      sorting,
      rowSelection,
    },
    onSortingChange: setSorting,
    onRowSelectionChange: setRowSelection,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
  });

  return table;
}
