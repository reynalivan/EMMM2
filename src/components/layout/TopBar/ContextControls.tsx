import { ShieldCheck, ShieldAlert } from 'lucide-react';
import { useAppStore } from '../../../stores/useAppStore';

export default function ContextControls() {
  const { safeMode, setSafeMode, activePreset, setActivePreset } = useAppStore();

  return (
    <div className="hidden lg:flex items-center gap-3 bg-base-100/30 p-1.5 rounded-full border border-white/5 backdrop-blur-md">
      <label
        className={`btn btn-xs btn-circle border-0 ${safeMode ? 'bg-success/20 text-success hover:bg-success/30' : 'bg-error/20 text-error hover:bg-error/30'}`}
        onClick={() => setSafeMode(!safeMode)}
        title="Safe Mode Toggle"
      >
        {safeMode ? <ShieldCheck size={14} /> : <ShieldAlert size={14} />}
      </label>

      <div className="h-4 w-px bg-white/10" />

      <div className="dropdown dropdown-bottom dropdown-end">
        <div
          tabIndex={0}
          role="button"
          className="px-3 py-1 rounded-full text-xs font-medium text-white/70 hover:text-white hover:bg-white/5 transition-all cursor-pointer flex items-center gap-2"
        >
          {activePreset || 'DefaultPreset'} <span className="text-[9px] opacity-30">▼</span>
        </div>
        <ul
          tabIndex={0}
          className="dropdown-content z-1 menu p-2 shadow-2xl bg-base-100/90 backdrop-blur-xl rounded-box w-40 mt-2 border border-white/10"
        >
          <li className="menu-title text-[10px] uppercase opacity-40 px-2 pb-1 tracking-widest">
            Loadouts
          </li>
          <li>
            <button className="hover:bg-white/5 text-sm" onClick={() => setActivePreset(null)}>
              Default
            </button>
          </li>
          <li>
            <button
              className="hover:bg-white/5 text-sm"
              onClick={() => setActivePreset('Exploration')}
            >
              Exploration
            </button>
          </li>
          <li>
            <button
              className="hover:bg-white/5 text-sm"
              onClick={() => setActivePreset('Photography')}
            >
              Photography
            </button>
          </li>
        </ul>
      </div>
    </div>
  );
}
