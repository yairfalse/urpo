/**
 * Tauri Integration Utilities
 * Handles graceful degradation when Tauri APIs are not available
 */

// Check if we're running in a Tauri environment
export const isTauriAvailable = (): boolean => {
  return typeof window !== 'undefined' && 
         window.__TAURI__ !== undefined;
};

// Safe wrapper for Tauri invoke calls
export const safeTauriInvoke = async <T>(
  command: string, 
  args?: Record<string, unknown>
): Promise<T | null> => {
  if (!isTauriAvailable()) {
    console.warn(`Tauri not available - skipping command: ${command}`);
    return null;
  }

  try {
    const { invoke } = await import('@tauri-apps/api/tauri');
    return await invoke<T>(command, args);
  } catch (error) {
    console.error(`Tauri command failed: ${command}`, error);
    return null;
  }
};

// Check if specific Tauri APIs are available
export const checkTauriFeatures = () => {
  return {
    isAvailable: isTauriAvailable(),
    canInvoke: isTauriAvailable(),
    // Add more feature checks as needed
  };
};

// Service Map API helpers
import type { ServiceMap } from '../types';

export interface ServiceMapOptions {
  limit?: number;
  time_window_seconds?: number;
}

export const getServiceMap = async (options?: ServiceMapOptions): Promise<ServiceMap | null> => {
  return await safeTauriInvoke<ServiceMap>('get_service_map', options);
};