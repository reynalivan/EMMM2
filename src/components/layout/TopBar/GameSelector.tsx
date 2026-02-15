import { Gamepad2 } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';

export default function GameSelector() {
  const { activeGame, setActiveGame } = useAppStore();

  return (
    <div className="dropdown dropdown-bottom">
      <div
        tabIndex={0}
        role="button"
        className="flex items-center gap-1.5 md:gap-2 px-2 md:px-3 py-1.5 rounded-full bg-base-200/30 border border-white/5 hover:border-primary/50 hover:bg-base-200/50 transition-all cursor-pointer group"
      >
        <div className="p-1 rounded-full bg-base-100/50 text-primary group-hover:text-white transition-colors">
          <Gamepad2 size={12} className="md:w-[14px] md:h-[14px]" />
        </div>
        <span className="hidden sm:inline text-sm font-medium text-white/90 group-hover:text-white">
          {activeGame === 'GIMI' && 'Genshin Impact'}
          {activeGame === 'SRMI' && 'Star Rail'}
          {activeGame === 'ZZMI' && 'Zenless Zone Zero'}
        </span>
        <span className="sm:hidden font-bold text-sm text-white/90">
          {activeGame === 'GIMI' && 'GI'}
          {activeGame === 'SRMI' && 'HSR'}
          {activeGame === 'ZZMI' && 'ZZZ'}
        </span>
        <span className="opacity-30 text-[10px] group-hover:opacity-100 transition-opacity">▼</span>
      </div>
      <ul
        tabIndex={0}
        className="dropdown-content z-1 menu p-2 shadow-xl bg-base-100/80 backdrop-blur-xl rounded-box w-52 border border-white/10 mt-2"
      >
        <li>
          <button
            onClick={() => setActiveGame('GIMI')}
            className={`hover:bg-white/5 ${activeGame === 'GIMI' ? 'text-primary font-bold bg-primary/10' : 'text-white/70'}`}
          >
            Genshin Impact
          </button>
        </li>
        <li>
          <button
            onClick={() => setActiveGame('SRMI')}
            className={`hover:bg-white/5 ${activeGame === 'SRMI' ? 'text-primary font-bold bg-primary/10' : 'text-white/70'}`}
          >
            Star Rail
          </button>
        </li>
        <li>
          <button
            onClick={() => setActiveGame('ZZMI')}
            className={`hover:bg-white/5 ${activeGame === 'ZZMI' ? 'text-primary font-bold bg-primary/10' : 'text-white/70'}`}
          >
            Zenless Zone Zero
          </button>
        </li>
      </ul>
    </div>
  );
}
