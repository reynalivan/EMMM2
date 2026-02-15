import { LayoutDashboard } from 'lucide-react';
import ScannerFeature from '../scanner/ScannerFeature';

export default function DashboardPlaceholder() {
  return (
    <div className="min-h-screen bg-base-100 p-6 space-y-8">
      {/* Header */}
      <div className="text-center space-y-4">
        <div className="mx-auto w-16 h-16 rounded-xl bg-linear-to-br from-primary/20 to-secondary/20 flex items-center justify-center">
          <LayoutDashboard className="w-8 h-8 text-primary" />
        </div>
        <div>
          <h1 className="text-2xl font-bold">Dashboard</h1>
          <p className="text-base-content/60">Your mod manager is ready.</p>
        </div>
      </div>

      {/* Scanner Feature */}
      <div className="max-w-4xl mx-auto">
        <ScannerFeature />
      </div>
    </div>
  );
}
