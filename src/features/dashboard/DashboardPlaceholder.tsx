import { LayoutDashboard } from 'lucide-react';

export default function DashboardPlaceholder() {
  return (
    <div className="min-h-screen bg-base-100 flex items-center justify-center p-6">
      <div className="text-center space-y-6">
        <div className="mx-auto w-20 h-20 rounded-2xl bg-linear-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
          <LayoutDashboard className="w-10 h-10 text-primary" />
        </div>
        <div>
          <h1 className="text-3xl font-bold">Dashboard</h1>
          <p className="text-base-content/60 mt-2">Setup complete! Your mod manager is ready.</p>
          <p className="text-base-content/40 text-sm mt-1">
            Dashboard features will be available in upcoming Epics.
          </p>
        </div>
      </div>
    </div>
  );
}
