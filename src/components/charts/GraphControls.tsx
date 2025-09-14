import { memo } from 'react';
import { LayoutMode } from '../../utils/graph/layouts';

interface GraphControlsProps {
  layoutMode: LayoutMode;
  showMetrics: boolean;
  onLayoutModeChange: (mode: LayoutMode) => void;
  onMetricsToggle: () => void;
  onRefresh: () => void;
}

const GraphControlsImpl = ({
  layoutMode,
  showMetrics,
  onLayoutModeChange,
  onMetricsToggle,
  onRefresh
}: GraphControlsProps) => {
  return (
    <div className="clean-card border-b border-surface-300 p-4 rounded-none">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h2 className="text-lg font-semibold text-text-900">Service Dependencies</h2>
          <div className="flex gap-2">
            {(['force', 'circular', 'hierarchical'] as LayoutMode[]).map(mode => (
              <button
                key={mode}
                onClick={() => onLayoutModeChange(mode)}
                className={`clean-button text-xs ${layoutMode === mode ? 'active' : ''}`}
              >
                {mode}
              </button>
            ))}
          </div>
        </div>
        
        <div className="flex items-center gap-4">
          <button
            onClick={onMetricsToggle}
            className={`clean-button text-xs ${showMetrics ? 'active' : ''}`}
          >
            Metrics
          </button>
          <button onClick={onRefresh} className="clean-button text-xs">
            Refresh
          </button>
        </div>
      </div>
    </div>
  );
};

export const GraphControls = memo(GraphControlsImpl);
GraphControls.displayName = 'GraphControls';