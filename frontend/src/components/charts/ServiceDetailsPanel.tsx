import { memo } from 'react';
import { ServiceNode } from '../../hooks/useDependencyDiscovery';

interface ServiceDetailsPanelProps {
  service: ServiceNode | undefined;
  serviceName: string;
  onClose?: () => void;
}

const ServiceDetailsPanelImpl = ({
  service,
  serviceName,
  onClose
}: ServiceDetailsPanelProps) => {
  if (!service) return null;

  return (
    <div className="absolute top-4 right-4 w-80 clean-card p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-semibold text-text-900">{serviceName}</h3>
        {onClose && (
          <button onClick={onClose} className="text-text-500 hover:text-text-900">
            ✕
          </button>
        )}
      </div>
      
      <div className="space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-text-500">Request Rate:</span>
          <span className="text-text-900 font-mono">
            {service.metrics.requestRate.toFixed(0)} req/s
          </span>
        </div>
        
        <div className="flex justify-between">
          <span className="text-text-500">Error Rate:</span>
          <span className={`font-mono ${
            service.metrics.errorRate > 5 ? 'text-status-error' : 
            service.metrics.errorRate > 1 ? 'text-status-warning' : 
            'text-text-900'
          }`}>
            {service.metrics.errorRate.toFixed(2)}%
          </span>
        </div>
        
        <div className="flex justify-between">
          <span className="text-text-500">P95 Latency:</span>
          <span className={`font-mono ${
            service.metrics.p95Latency > 200 ? 'text-status-warning' : 'text-text-900'
          }`}>
            {service.metrics.p95Latency.toFixed(0)}ms
          </span>
        </div>
        
        <div className="flex justify-between">
          <span className="text-text-500">Active Spans:</span>
          <span className="text-text-900 font-mono">
            {service.metrics.spanCount.toLocaleString()}
          </span>
        </div>
      </div>
      
      <div className="mt-4 pt-4 border-t border-surface-300">
        <div className="text-xs text-text-500">
          Position: ({service.x.toFixed(0)}, {service.y.toFixed(0)})
        </div>
      </div>
    </div>
  );
};

export const ServiceDetailsPanel = memo(ServiceDetailsPanelImpl);
ServiceDetailsPanel.displayName = 'ServiceDetailsPanel';