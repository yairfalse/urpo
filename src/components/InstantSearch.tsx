// INSTANT SEARCH - FASTER THAN YOUR BRAIN CAN PROCESS
import React, { useState, useCallback, useEffect, useMemo, memo } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { TraceInfo } from '../types';

interface SearchProps {
  onTraceSelect: (trace: TraceInfo) => void;
}

// Debounce hook for smooth typing
function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [value, delay]);

  return debouncedValue;
}

const InstantSearch = memo(({ onTraceSelect }: SearchProps) => {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<TraceInfo[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [searchTime, setSearchTime] = useState<number>(0);
  const [filters, setFilters] = useState({
    service: '',
    errorOnly: false,
    lastHour: false,
    last15Min: false,
  });

  // Debounce search query for smooth UX
  const debouncedQuery = useDebounce(query, 150); // 150ms for instant feel

  // BLAZING FAST search
  const performSearch = useCallback(async (searchQuery: string) => {
    if (!searchQuery && !filters.service && !filters.errorOnly) {
      setResults([]);
      return;
    }

    setIsSearching(true);
    const startTime = performance.now();

    try {
      const searchResults = await invoke<TraceInfo[]>('search_traces', {
        query: searchQuery,
        limit: 100,
        serviceFilter: filters.service || null,
        errorOnly: filters.errorOnly,
        timeRange: filters.last15Min ? 15 : filters.lastHour ? 60 : null,
      });

      const searchDuration = performance.now() - startTime;
      setSearchTime(searchDuration);
      setResults(searchResults);
      setSelectedIndex(0);
    } catch (error) {
      console.error('Search failed:', error);
      setResults([]);
    } finally {
      setIsSearching(false);
    }
  }, [filters]);

  // Auto-search on query or filter change
  useEffect(() => {
    performSearch(debouncedQuery);
  }, [debouncedQuery, filters, performSearch]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, results.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, 0));
      } else if (e.key === 'Enter' && results[selectedIndex]) {
        e.preventDefault();
        onTraceSelect(results[selectedIndex]);
      } else if (e.key === 'Escape') {
        setQuery('');
        setResults([]);
      } else if (e.metaKey && e.key === 'k') {
        e.preventDefault();
        document.getElementById('search-input')?.focus();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, selectedIndex, onTraceSelect]);

  // Format duration for display
  const formatDuration = useCallback((duration: number) => {
    if (duration < 1000) return `${duration}μs`;
    if (duration < 1000000) return `${(duration / 1000).toFixed(1)}ms`;
    return `${(duration / 1000000).toFixed(2)}s`;
  }, []);

  // Format time ago
  const formatTimeAgo = useCallback((timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  }, []);

  return (
    <div className="instant-search">
      {/* Search Header */}
      <div className="search-header bg-gray-900 p-4 border-b border-gray-800">
        <div className="flex items-center gap-4">
          {/* Main Search Input */}
          <div className="flex-1 relative">
            <input
              id="search-input"
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search traces... (⌘K)"
              className="w-full px-4 py-2 pl-10 bg-gray-800 text-white rounded-lg 
                       border border-gray-700 focus:border-blue-500 focus:outline-none
                       placeholder-gray-500"
              autoFocus
            />
            <svg className="absolute left-3 top-2.5 w-5 h-5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            {isSearching && (
              <div className="absolute right-3 top-3">
                <div className="animate-spin h-4 w-4 border-2 border-blue-500 rounded-full border-t-transparent"></div>
              </div>
            )}
          </div>

          {/* Quick Filters */}
          <div className="flex gap-2">
            <button
              onClick={() => setFilters(f => ({ ...f, errorOnly: !f.errorOnly }))}
              className={`px-3 py-1 rounded-md text-sm font-medium transition-colors
                ${filters.errorOnly 
                  ? 'bg-red-600 text-white' 
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'}`}
            >
              Errors Only
            </button>
            <button
              onClick={() => setFilters(f => ({ ...f, last15Min: !f.last15Min, lastHour: false }))}
              className={`px-3 py-1 rounded-md text-sm font-medium transition-colors
                ${filters.last15Min 
                  ? 'bg-blue-600 text-white' 
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'}`}
            >
              Last 15m
            </button>
            <button
              onClick={() => setFilters(f => ({ ...f, lastHour: !f.lastHour, last15Min: false }))}
              className={`px-3 py-1 rounded-md text-sm font-medium transition-colors
                ${filters.lastHour 
                  ? 'bg-blue-600 text-white' 
                  : 'bg-gray-800 text-gray-400 hover:bg-gray-700'}`}
            >
              Last Hour
            </button>
          </div>
        </div>

        {/* Search Stats */}
        {results.length > 0 && (
          <div className="mt-2 text-xs text-gray-500">
            Found {results.length} traces in {searchTime.toFixed(1)}ms
            {searchTime < 10 && <span className="ml-2 text-green-500">⚡ BLAZING FAST!</span>}
          </div>
        )}
      </div>

      {/* Search Results */}
      <div className="search-results overflow-y-auto max-h-[600px] bg-gray-950">
        {results.map((trace, index) => (
          <div
            key={trace.trace_id}
            onClick={() => onTraceSelect(trace)}
            onMouseEnter={() => setSelectedIndex(index)}
            className={`p-3 border-b border-gray-900 cursor-pointer transition-all
              ${index === selectedIndex 
                ? 'bg-gray-800 border-l-4 border-blue-500' 
                : 'hover:bg-gray-900'}`}
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                {/* Trace Header */}
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-sm font-mono text-blue-400">
                    {trace.trace_id.substring(0, 16)}...
                  </span>
                  {trace.has_error && (
                    <span className="px-2 py-0.5 bg-red-600 text-white text-xs rounded">
                      ERROR
                    </span>
                  )}
                  <span className="text-xs text-gray-500">
                    {formatTimeAgo(trace.start_time * 1000)}
                  </span>
                </div>

                {/* Service & Operation */}
                <div className="flex items-center gap-2 text-sm">
                  <span className="text-green-400">{trace.root_service}</span>
                  <span className="text-gray-600">→</span>
                  <span className="text-gray-300">{trace.root_operation}</span>
                </div>

                {/* Trace Metadata */}
                <div className="flex items-center gap-4 mt-1 text-xs text-gray-500">
                  <span>{trace.span_count} spans</span>
                  <span>{trace.services.length} services</span>
                  <span className={`font-medium ${
                    trace.duration < 100 ? 'text-green-500' :
                    trace.duration < 500 ? 'text-yellow-500' :
                    'text-red-500'
                  }`}>
                    {formatDuration(trace.duration)}
                  </span>
                </div>
              </div>

              {/* Visual Indicator */}
              <div className="ml-4">
                <div className={`w-2 h-2 rounded-full ${
                  trace.has_error ? 'bg-red-500' :
                  trace.duration > 1000 ? 'bg-yellow-500' :
                  'bg-green-500'
                }`}></div>
              </div>
            </div>
          </div>
        ))}

        {/* Empty State */}
        {!isSearching && query && results.length === 0 && (
          <div className="p-8 text-center text-gray-500">
            <svg className="mx-auto h-12 w-12 text-gray-700 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} 
                    d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M12 12h.01M12 12h-.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <p>No traces found for "{query}"</p>
            <p className="text-sm mt-1">Try a different search term or adjust filters</p>
          </div>
        )}

        {/* Initial State */}
        {!query && !filters.errorOnly && (
          <div className="p-8 text-center text-gray-600">
            <div className="space-y-2 text-sm">
              <p>🔍 Start typing to search traces</p>
              <p className="text-xs">Try: service name, operation, trace ID, or "error"</p>
              
              <div className="mt-4 pt-4 border-t border-gray-800 text-left max-w-xs mx-auto">
                <p className="font-semibold mb-2">Keyboard Shortcuts:</p>
                <div className="space-y-1 text-xs">
                  <div><kbd>⌘K</kbd> - Focus search</div>
                  <div><kbd>↑↓</kbd> - Navigate results</div>
                  <div><kbd>Enter</kbd> - Open trace</div>
                  <div><kbd>Esc</kbd> - Clear search</div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
});

InstantSearch.displayName = 'InstantSearch';

export default InstantSearch;