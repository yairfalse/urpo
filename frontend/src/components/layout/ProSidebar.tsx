import {
  Activity,
  GitBranch,
  Layers,
  BarChart3,
  Share2,
  Settings,
  HelpCircle,
  TrendingUp,
  Cpu,
  Database,
  Cloud,
  Shield,
  Zap,
  Bell,
  Search,
  Terminal
} from 'lucide-react';

interface NavItem {
  id: string;
  label: string;
  icon: any;
  badge?: string;
  badgeColor?: string;
  children?: NavItem[];
}

interface ProSidebarProps {
  activeView: string;
  onViewChange: (view: string) => void;
}

export const ProSidebar = ({ activeView, onViewChange }: ProSidebarProps) => {
  const navSections = [
    {
      title: 'OBSERVABILITY',
      items: [
        { id: 'graph', label: 'Service Map', icon: GitBranch, badge: 'Live', badgeColor: 'bg-semantic-success' },
        { id: 'flows', label: 'Trace Flow', icon: Activity },
        { id: 'traces', label: 'Trace Explorer', icon: Layers, badge: '23k', badgeColor: 'bg-data-blue' },
        { id: 'health', label: 'Health Matrix', icon: BarChart3 },
        { id: 'dependencies', label: 'Dependencies', icon: Share2 },
      ]
    },
    {
      title: 'PERFORMANCE',
      items: [
        { id: 'latency', label: 'Latency Analysis', icon: TrendingUp },
        { id: 'throughput', label: 'Throughput', icon: Zap },
        { id: 'resources', label: 'Resources', icon: Cpu },
        { id: 'database', label: 'Database', icon: Database },
      ]
    },
    {
      title: 'INFRASTRUCTURE',
      items: [
        { id: 'services', label: 'Services', icon: Cloud },
        { id: 'security', label: 'Security', icon: Shield },
        { id: 'alerts', label: 'Alerts', icon: Bell, badge: '3', badgeColor: 'bg-semantic-error' },
        { id: 'logs', label: 'Logs', icon: Terminal },
      ]
    },
  ];

  return (
    <div className="h-full flex flex-col">
      {/* Search in sidebar */}
      <div className="p-4 border-b border-dark-300/50">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-light-500" />
          <input
            type="text"
            placeholder="Quick find..."
            className="
              w-full pl-9 pr-3 py-2
              bg-dark-200/30
              border border-dark-400/30
              rounded-lg
              text-sm text-light-300
              placeholder-light-600
              focus:outline-none focus:ring-1 focus:ring-data-blue/50
              focus:border-data-blue/50
              transition-all
            "
          />
        </div>
      </div>

      {/* Navigation sections */}
      <nav className="flex-1 overflow-y-auto px-3 py-4">
        {navSections.map((section, idx) => (
          <div key={idx} className="mb-6">
            {/* Section title */}
            <h3 className="
              px-2 mb-2
              text-[10px] font-semibold uppercase tracking-wider
              text-light-600
            ">
              {section.title}
            </h3>

            {/* Section items */}
            <div className="space-y-1">
              {section.items.map((item) => {
                const Icon = item.icon;
                const isActive = activeView === item.id;

                return (
                  <button
                    key={item.id}
                    onClick={() => onViewChange(item.id)}
                    className={`
                      w-full px-3 py-2.5
                      flex items-center justify-between
                      rounded-lg
                      text-sm font-medium
                      transition-all duration-150
                      group
                      ${isActive
                        ? 'bg-gradient-to-r from-data-blue/20 to-data-purple/20 text-light-100 border border-data-blue/30'
                        : 'text-light-400 hover:text-light-100 hover:bg-dark-200/30'
                      }
                    `}
                  >
                    <div className="flex items-center gap-3">
                      <div className={`
                        p-1.5 rounded-lg
                        ${isActive
                          ? 'bg-gradient-to-br from-data-blue to-data-purple shadow-lg'
                          : 'bg-dark-300/30 group-hover:bg-dark-300/50'
                        }
                        transition-all
                      `}>
                        <Icon className={`w-4 h-4 ${isActive ? 'text-white' : 'text-light-400'}`} />
                      </div>
                      <span>{item.label}</span>
                    </div>

                    {item.badge && (
                      <span className={`
                        px-2 py-0.5
                        text-[10px] font-bold
                        rounded-full
                        ${item.badgeColor || 'bg-dark-300'}
                        ${item.badgeColor?.includes('error') ? 'text-white' : 'text-light-100'}
                      `}>
                        {item.badge}
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      {/* Bottom section */}
      <div className="p-4 border-t border-dark-300/50">
        <button className="
          w-full px-3 py-2
          flex items-center gap-3
          text-sm text-light-400
          hover:text-light-200
          hover:bg-dark-200/30
          rounded-lg
          transition-all
        ">
          <Settings className="w-4 h-4" />
          <span>Settings</span>
        </button>
        <button className="
          w-full px-3 py-2 mt-1
          flex items-center gap-3
          text-sm text-light-400
          hover:text-light-200
          hover:bg-dark-200/30
          rounded-lg
          transition-all
        ">
          <HelpCircle className="w-4 h-4" />
          <span>Help & Docs</span>
        </button>
      </div>
    </div>
  );
};