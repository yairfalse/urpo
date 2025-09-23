import { ReactNode } from 'react';
import {
  Activity,
  Bell,
  Settings,
  Search,
  Filter,
  RefreshCw,
  Clock,
  TrendingUp,
  AlertTriangle,
  CheckCircle,
  XCircle,
  Info,
  HelpCircle,
  Moon,
  Sun,
  Maximize2,
  Grid3x3
} from 'lucide-react';

interface ProfessionalLayoutProps {
  children: ReactNode;
  sidebar?: ReactNode;
  header?: ReactNode;
}

export const ProfessionalLayout = ({ children, sidebar, header }: ProfessionalLayoutProps) => {
  return (
    <div className="h-screen bg-gradient-to-br from-dark-0 via-dark-50 to-dark-0 flex flex-col">
      {/* Ultra-modern header with glass effect */}
      <header className="relative bg-dark-100/80 backdrop-blur-xl border-b border-dark-300/50">
        {/* Gradient accent line */}
        <div className="absolute top-0 left-0 right-0 h-[2px] bg-gradient-to-r from-data-purple via-data-blue to-data-cyan"></div>

        <div className="px-6 py-3">
          <div className="flex items-center justify-between">
            {/* Left section - Brand and Navigation */}
            <div className="flex items-center gap-8">
              {/* Logo with gradient */}
              <div className="flex items-center gap-3">
                <div className="relative">
                  <div className="absolute inset-0 bg-gradient-to-br from-data-blue to-data-cyan blur-lg opacity-50"></div>
                  <div className="relative w-10 h-10 bg-gradient-to-br from-data-blue to-data-cyan rounded-xl flex items-center justify-center shadow-lg">
                    <Activity className="w-6 h-6 text-white" />
                  </div>
                </div>
                <div>
                  <h1 className="text-xl font-bold bg-gradient-to-r from-light-50 to-light-200 bg-clip-text text-transparent">
                    URPO
                  </h1>
                  <p className="text-[10px] text-light-500 uppercase tracking-[0.2em] font-medium">
                    Trace Analytics
                  </p>
                </div>
              </div>

              {/* Search with advanced styling */}
              <div className="relative group">
                <div className="absolute inset-0 bg-gradient-to-r from-data-blue/20 to-data-purple/20 rounded-lg blur-xl opacity-0 group-hover:opacity-100 transition-opacity"></div>
                <div className="relative flex items-center">
                  <Search className="absolute left-3 w-4 h-4 text-light-500" />
                  <input
                    type="text"
                    placeholder="Search services, traces, errors..."
                    className="
                      w-96 pl-10 pr-4 py-2
                      bg-dark-200/50 backdrop-blur
                      border border-dark-400/50
                      rounded-lg
                      text-light-200 placeholder-light-500
                      focus:outline-none focus:ring-2 focus:ring-data-blue/50
                      focus:border-data-blue/50
                      transition-all
                    "
                  />
                  <div className="absolute right-3 flex items-center gap-1">
                    <kbd className="px-1.5 py-0.5 text-[10px] bg-dark-300/50 text-light-500 rounded border border-dark-400/50">⌘K</kbd>
                  </div>
                </div>
              </div>
            </div>

            {/* Right section - Status and Actions */}
            <div className="flex items-center gap-6">
              {/* Live status indicators */}
              <div className="flex items-center gap-4 px-4 py-1.5 bg-dark-200/30 rounded-lg backdrop-blur">
                <div className="flex items-center gap-2">
                  <div className="relative">
                    <div className="absolute inset-0 bg-semantic-success blur-sm animate-pulse"></div>
                    <div className="relative w-2 h-2 bg-semantic-success rounded-full"></div>
                  </div>
                  <span className="text-xs text-light-400">Live</span>
                </div>
                <div className="h-4 w-px bg-dark-400"></div>
                <div className="flex items-center gap-2">
                  <TrendingUp className="w-3.5 h-3.5 text-data-cyan" />
                  <span className="text-xs font-medium text-light-300">23.4k/s</span>
                </div>
                <div className="h-4 w-px bg-dark-400"></div>
                <div className="flex items-center gap-2">
                  <Clock className="w-3.5 h-3.5 text-light-500" />
                  <span className="text-xs text-light-400">15m</span>
                </div>
              </div>

              {/* Action buttons with better styling */}
              <div className="flex items-center gap-2">
                <button className="
                  p-2 rounded-lg
                  bg-dark-200/30 backdrop-blur
                  text-light-400 hover:text-light-200
                  hover:bg-dark-200/50
                  transition-all
                  group
                ">
                  <Filter className="w-4 h-4 group-hover:scale-110 transition-transform" />
                </button>
                <button className="
                  p-2 rounded-lg
                  bg-dark-200/30 backdrop-blur
                  text-light-400 hover:text-light-200
                  hover:bg-dark-200/50
                  transition-all
                  group
                ">
                  <RefreshCw className="w-4 h-4 group-hover:rotate-180 transition-transform duration-500" />
                </button>
                <button className="
                  relative p-2 rounded-lg
                  bg-dark-200/30 backdrop-blur
                  text-light-400 hover:text-light-200
                  hover:bg-dark-200/50
                  transition-all
                ">
                  <Bell className="w-4 h-4" />
                  <span className="absolute top-1 right-1 w-2 h-2 bg-semantic-error rounded-full"></span>
                </button>
                <button className="
                  p-2 rounded-lg
                  bg-dark-200/30 backdrop-blur
                  text-light-400 hover:text-light-200
                  hover:bg-dark-200/50
                  transition-all
                ">
                  <Settings className="w-4 h-4" />
                </button>
              </div>

              {/* User section */}
              <div className="flex items-center gap-3 pl-6 border-l border-dark-300/50">
                <div className="text-right">
                  <p className="text-xs font-medium text-light-200">Admin User</p>
                  <p className="text-[10px] text-light-500">Production</p>
                </div>
                <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-data-purple to-data-pink shadow-lg"></div>
              </div>
            </div>
          </div>
        </div>

        {header}
      </header>

      {/* Main content area with sidebar */}
      <div className="flex-1 flex overflow-hidden">
        {sidebar && (
          <aside className="w-64 bg-dark-100/50 backdrop-blur-sm border-r border-dark-300/50">
            {sidebar}
          </aside>
        )}

        {/* Content area with subtle grid background */}
        <main className="flex-1 relative overflow-auto">
          {/* Grid background pattern */}
          <div className="absolute inset-0 bg-grid-pattern opacity-[0.02] pointer-events-none"></div>
          {children}
        </main>
      </div>

      {/* Professional status bar */}
      <footer className="
        relative bg-dark-100/80 backdrop-blur-xl
        border-t border-dark-300/50
        px-6 py-2
      ">
        <div className="flex items-center justify-between">
          {/* Left - Quick stats */}
          <div className="flex items-center gap-6">
            <div className="flex items-center gap-2">
              <CheckCircle className="w-3.5 h-3.5 text-semantic-success" />
              <span className="text-xs text-light-400">
                Services: <span className="font-medium text-light-200">42</span>
              </span>
            </div>
            <div className="flex items-center gap-2">
              <AlertTriangle className="w-3.5 h-3.5 text-semantic-warning" />
              <span className="text-xs text-light-400">
                Warnings: <span className="font-medium text-semantic-warning">3</span>
              </span>
            </div>
            <div className="flex items-center gap-2">
              <XCircle className="w-3.5 h-3.5 text-semantic-error" />
              <span className="text-xs text-light-400">
                Errors: <span className="font-medium text-semantic-error">0</span>
              </span>
            </div>
          </div>

          {/* Right - System info */}
          <div className="flex items-center gap-4 text-xs text-light-500">
            <span>v1.0.0</span>
            <span>•</span>
            <span>Region: US-WEST</span>
            <span>•</span>
            <span>Cluster: PROD-01</span>
            <span>•</span>
            <button className="hover:text-light-300 transition-colors">
              <HelpCircle className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </footer>
    </div>
  );
};