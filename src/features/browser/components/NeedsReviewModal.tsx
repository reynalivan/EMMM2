import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import type { ImportJobItem } from '../types';
import type { GameConfig } from '../../../types/game';

interface Props {
  job: ImportJobItem;
  open: boolean;
  onClose: () => void;
  onConfirm: (gameId: string, category: string, objectId?: string | null) => void;
  onSkip: () => void;
}

const CATEGORIES = ['Character', 'Weapon', 'UI', 'Other'] as const;

export function NeedsReviewModal({ job, open, onClose, onConfirm, onSkip }: Props) {
  const [selectedGameId, setSelectedGameId] = useState<string>(job.game_id ?? '');
  const [selectedCategory, setSelectedCategory] = useState<string>(job.match_category ?? 'Other');

  const gamesQuery = useQuery({
    queryKey: ['games'],
    queryFn: () => invoke<GameConfig[]>('get_games'),
    enabled: open,
  });

  const games = gamesQuery.data ?? [];

  if (!open) return null;

  return (
    <dialog
      id="needs-review-modal"
      className="modal modal-open"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="modal-box max-w-md">
        {/* Header */}
        <div className="flex items-start gap-3 mb-4">
          <div className="text-warning mt-0.5">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="w-6 h-6"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
          </div>
          <div>
            <h3 className="font-bold text-lg leading-tight">Manual Review Required</h3>
            <p className="text-sm text-base-content/60 mt-1">{job.match_reason}</p>
          </div>
        </div>

        {/* Archive info */}
        <div className="bg-base-300 rounded-lg p-3 mb-4">
          <p className="text-xs text-base-content/50 mb-1">Archive</p>
          <p className="text-sm font-mono truncate">{job.archive_path.split(/[/\\]/).pop()}</p>
          {job.is_duplicate && <div className="badge badge-warning badge-sm mt-2">Duplicate</div>}
        </div>

        {/* Game Picker */}
        <div className="form-control mb-4">
          <label className="label py-1">
            <span className="label-text font-medium">Target Game</span>
          </label>
          <select
            id="review-game-select"
            className="select select-bordered select-sm"
            value={selectedGameId}
            onChange={(e) => setSelectedGameId(e.target.value)}
          >
            <option value="">— Select game —</option>
            {games.map((g) => (
              <option key={g.id} value={g.id}>
                {g.name}
              </option>
            ))}
          </select>
        </div>

        {/* Category Picker */}
        <div className="form-control mb-4">
          <label className="label py-1">
            <span className="label-text font-medium">Mod Category</span>
          </label>
          <div className="flex gap-2 flex-wrap">
            {CATEGORIES.map((cat) => (
              <label key={cat} className="cursor-pointer">
                <input
                  type="radio"
                  name="review-category-radio"
                  className="hidden"
                  value={cat}
                  checked={selectedCategory === cat}
                  onChange={() => setSelectedCategory(cat)}
                />
                <span
                  className={`
                    badge badge-md cursor-pointer transition-colors
                    ${
                      selectedCategory === cat ? 'badge-primary' : 'badge-ghost hover:badge-outline'
                    }
                  `}
                >
                  {cat}
                </span>
              </label>
            ))}
          </div>
        </div>

        <div className="modal-action mt-2">
          <button className="btn btn-ghost btn-sm" onClick={onSkip}>
            Skip
          </button>
          <button className="btn btn-ghost btn-sm" onClick={onClose}>
            Cancel
          </button>
          <button
            id="review-confirm-btn"
            className="btn btn-primary btn-sm"
            disabled={!selectedGameId || !selectedCategory}
            onClick={() => onConfirm(selectedGameId, selectedCategory, null)}
          >
            Confirm & Import
          </button>
        </div>
      </div>
    </dialog>
  );
}
