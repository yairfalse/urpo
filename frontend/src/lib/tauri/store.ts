/**
 * Zustand store for global application state
 *
 * This store manages:
 * - UI state (active view, filters, etc.)
 * - Selected entities (traces, services)
 * - User preferences
 * - Performance metrics
 */

import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { ServiceMetrics, TraceInfo } from './types';

// ============================================================================
// TYPES
// ============================================================================

export type ViewMode = 'graph' | 'flows' | 'health' | 'traces' | 'servicemap';
export type ServiceMapViewMode = 'topology' | 'focus' | 'hotpaths' | 'errors';
export type ColorScheme = 'dark' | 'light' | 'system';

interface FilterState {
  serviceFilter?: string;
  statusFilter?: 'all' | 'healthy' | 'warning' | 'error';
  timeRange: '15m' | '1h' | '6h' | '24h' | '7d';
}

interface UIState {
  activeView: ViewMode;
  serviceMapView: ServiceMapViewMode;
  selectedService: string | null;
  selectedTrace: string | null;
  searchQuery: string;
  showFilters: boolean;
  showNotifications: boolean;
  showSettings: boolean;
  showUserMenu: boolean;
  colorScheme: ColorScheme;
  filters: FilterState;
}

interface PerformanceMetrics {
  frameRate: number;
  renderTime: number;
  lastUpdate: number;
}

interface AppState extends UIState {
  // Performance tracking
  performance: PerformanceMetrics;

  // Actions
  setActiveView: (view: ViewMode) => void;
  setServiceMapView: (view: ServiceMapViewMode) => void;
  selectService: (service: string | null) => void;
  selectTrace: (trace: string | null) => void;
  setSearchQuery: (query: string) => void;
  toggleFilters: () => void;
  toggleNotifications: () => void;
  toggleSettings: () => void;
  toggleUserMenu: () => void;
  setColorScheme: (scheme: ColorScheme) => void;
  updateFilters: (filters: Partial<FilterState>) => void;
  updatePerformance: (metrics: Partial<PerformanceMetrics>) => void;
  resetUI: () => void;
  closeAllDropdowns: () => void;
}

// ============================================================================
// DEFAULT STATE
// ============================================================================

const defaultUIState: UIState = {
  activeView: 'graph',
  serviceMapView: 'topology',
  selectedService: null,
  selectedTrace: null,
  searchQuery: '',
  showFilters: false,
  showNotifications: false,
  showSettings: false,
  showUserMenu: false,
  colorScheme: 'dark',
  filters: {
    timeRange: '15m',
    statusFilter: 'all',
  },
};

const defaultPerformance: PerformanceMetrics = {
  frameRate: 60,
  renderTime: 0,
  lastUpdate: Date.now(),
};

// ============================================================================
// STORE
// ============================================================================

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      // Initial state
      ...defaultUIState,
      performance: defaultPerformance,

      // View management
      setActiveView: (view) => set({ activeView: view }),
      setServiceMapView: (view) => set({ serviceMapView: view }),

      // Selection management
      selectService: (service) => set({ selectedService: service }),
      selectTrace: (trace) => set({ selectedTrace: trace }),

      // Search
      setSearchQuery: (query) => set({ searchQuery: query }),

      // UI toggles
      toggleFilters: () => set((state) => ({
        showFilters: !state.showFilters,
        // Close other dropdowns
        showNotifications: false,
        showSettings: false,
        showUserMenu: false,
      })),

      toggleNotifications: () => set((state) => ({
        showNotifications: !state.showNotifications,
        // Close other dropdowns
        showFilters: false,
        showSettings: false,
        showUserMenu: false,
      })),

      toggleSettings: () => set((state) => ({
        showSettings: !state.showSettings,
        // Close other dropdowns
        showFilters: false,
        showNotifications: false,
        showUserMenu: false,
      })),

      toggleUserMenu: () => set((state) => ({
        showUserMenu: !state.showUserMenu,
        // Close other dropdowns
        showFilters: false,
        showNotifications: false,
        showSettings: false,
      })),

      // Close all dropdowns
      closeAllDropdowns: () => set({
        showFilters: false,
        showNotifications: false,
        showSettings: false,
        showUserMenu: false,
      }),

      // Preferences
      setColorScheme: (scheme) => set({ colorScheme: scheme }),

      // Filters
      updateFilters: (filters) => set((state) => ({
        filters: { ...state.filters, ...filters },
      })),

      // Performance
      updatePerformance: (metrics) => set((state) => ({
        performance: { ...state.performance, ...metrics, lastUpdate: Date.now() },
      })),

      // Reset
      resetUI: () => set({ ...defaultUIState, performance: defaultPerformance }),
    }),
    {
      name: 'urpo-app-state',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        // Only persist user preferences
        activeView: state.activeView,
        serviceMapView: state.serviceMapView,
        colorScheme: state.colorScheme,
        filters: state.filters,
      }),
    }
  )
);

// ============================================================================
// SELECTORS
// ============================================================================

/**
 * Select only UI-related state
 */
export const useUIState = () => {
  return useAppStore((state) => ({
    activeView: state.activeView,
    serviceMapView: state.serviceMapView,
    selectedService: state.selectedService,
    selectedTrace: state.selectedTrace,
    searchQuery: state.searchQuery,
    showFilters: state.showFilters,
    showNotifications: state.showNotifications,
    showSettings: state.showSettings,
    showUserMenu: state.showUserMenu,
  }));
};

/**
 * Select only filter state
 */
export const useFilters = () => {
  return useAppStore((state) => state.filters);
};

/**
 * Select only performance metrics
 */
export const usePerformanceMetrics = () => {
  return useAppStore((state) => state.performance);
};

// ============================================================================
// UTILITIES
// ============================================================================

/**
 * Helper to check if a service has errors based on threshold
 */
export function hasServiceError(service: ServiceMetrics): boolean {
  return service.error_rate > 0.01; // 1% error rate threshold
}

/**
 * Helper to check if a service has warnings
 */
export function hasServiceWarning(service: ServiceMetrics): boolean {
  return service.error_rate > 0.001 && service.error_rate <= 0.01;
}

/**
 * Helper to get service health status
 */
export function getServiceHealth(service: ServiceMetrics): 'healthy' | 'warning' | 'error' {
  if (hasServiceError(service)) return 'error';
  if (hasServiceWarning(service)) return 'warning';
  return 'healthy';
}

/**
 * Helper to format time range for display
 */
export function formatTimeRange(range: FilterState['timeRange']): string {
  switch (range) {
    case '15m': return 'Last 15 minutes';
    case '1h': return 'Last hour';
    case '6h': return 'Last 6 hours';
    case '24h': return 'Last 24 hours';
    case '7d': return 'Last 7 days';
    default: return range;
  }
}

/**
 * Helper to get milliseconds for time range
 */
export function getTimeRangeMs(range: FilterState['timeRange']): number {
  switch (range) {
    case '15m': return 15 * 60 * 1000;
    case '1h': return 60 * 60 * 1000;
    case '6h': return 6 * 60 * 60 * 1000;
    case '24h': return 24 * 60 * 60 * 1000;
    case '7d': return 7 * 24 * 60 * 60 * 1000;
    default: return 15 * 60 * 1000;
  }
}