// COMMAND PALETTE - POWER USER MODE ACTIVATED
import React, { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface Command {
  id: string;
  label: string;
  shortcut?: string;
  icon?: string;
  action: () => void | Promise<void>;
  category: 'search' | 'filter' | 'view' | 'export' | 'compare';
}

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onCommand?: (command: Command) => void;
}

const CommandPalette: React.FC<CommandPaletteProps> = ({ isOpen, onClose, onCommand }) => {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [results, setResults] = useState<Command[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  // Natural language parser
  const parseNaturalQuery = useCallback((input: string): Command | null => {
    const lower = input.toLowerCase();
    
    // Error queries
    if (lower.includes('error') || lower.includes('fail')) {
      const timeMatch = lower.match(/last (\d+)([mhs])/);
      const time = timeMatch ? parseInt(timeMatch[1]) : 5;
      const unit = timeMatch ? timeMatch[2] : 'm';
      
      return {
        id: 'errors',
        label: `Show errors in last ${time}${unit}`,
        icon: '🔴',
        category: 'search',
        action: async () => {
          await invoke('search_traces', { 
            query: 'error',
            timeRange: unit === 'm' ? time : unit === 'h' ? time * 60 : time * 3600,
          });
        }
      };
    }
    
    // Service filter
    if (lower.includes('service:')) {
      const service = lower.split('service:')[1].trim().split(' ')[0];
      return {
        id: 'service-filter',
        label: `Filter by service: ${service}`,
        icon: '🎯',
        category: 'filter',
        action: async () => {
          await invoke('search_traces', { serviceFilter: service });
        }
      };
    }
    
    // Slow queries
    if (lower.includes('slow') || lower.includes('>')) {
      const msMatch = lower.match(/>(\d+)ms/) || lower.match(/(\d+)ms/);
      const threshold = msMatch ? parseInt(msMatch[1]) : 100;
      
      return {
        id: 'slow-traces',
        label: `Find traces slower than ${threshold}ms`,
        icon: '🐌',
        category: 'search',
        action: async () => {
          await invoke('search_traces', { 
            minDuration: threshold * 1000 // Convert to microseconds
          });
        }
      };
    }
    
    // Compare traces
    if (lower.includes('compare')) {
      const traceIds = lower.match(/[a-f0-9]{16}/g);
      if (traceIds && traceIds.length >= 2) {
        return {
          id: 'compare',
          label: `Compare traces ${traceIds[0].substring(0, 8)}... with ${traceIds[1].substring(0, 8)}...`,
          icon: '⚖️',
          category: 'compare',
          action: async () => {
            await invoke('compare_traces', { 
              traceA: traceIds[0],
              traceB: traceIds[1]
            });
          }
        };
      }
    }
    
    return null;
  }, []);

  // Built-in commands
  const builtInCommands: Command[] = [
    {
      id: 'show-live-map',
      label: 'Show Live Service Map',
      shortcut: '⌘L',
      icon: '🗺️',
      category: 'view',
      action: () => {
        window.dispatchEvent(new CustomEvent('show-view', { detail: 'map' }));
      }
    },
    {
      id: 'show-heatmap',
      label: 'Show Latency Heatmap',
      shortcut: '⌘H',
      icon: '🔥',
      category: 'view',
      action: () => {
        window.dispatchEvent(new CustomEvent('show-view', { detail: 'heatmap' }));
      }
    },
    {
      id: 'split-view',
      label: 'Split View Mode',
      shortcut: '⌘\\',
      icon: '⚡',
      category: 'view',
      action: () => {
        window.dispatchEvent(new CustomEvent('toggle-split-view'));
      }
    },
    {
      id: 'export-jaeger',
      label: 'Export to Jaeger Format',
      icon: '📤',
      category: 'export',
      action: async () => {
        await invoke('export_traces', { format: 'jaeger' });
      }
    },
    {
      id: 'set-baseline',
      label: 'Set Current as Baseline',
      shortcut: '⌘B',
      icon: '📍',
      category: 'compare',
      action: async () => {
        await invoke('set_baseline');
      }
    },
    {
      id: 'clear-all',
      label: 'Clear All Traces',
      shortcut: '⌘⇧K',
      icon: '🗑️',
      category: 'filter',
      action: async () => {
        await invoke('clear_traces');
      }
    },
  ];

  // Filter commands based on query
  useEffect(() => {
    if (!query) {
      setResults(builtInCommands);
      return;
    }

    const filtered: Command[] = [];
    
    // Try natural language parsing first
    const naturalCommand = parseNaturalQuery(query);
    if (naturalCommand) {
      filtered.push(naturalCommand);
    }
    
    // Then filter built-in commands
    const queryLower = query.toLowerCase();
    builtInCommands.forEach(cmd => {
      if (cmd.label.toLowerCase().includes(queryLower) ||
          cmd.category.includes(queryLower)) {
        filtered.push(cmd);
      }
    });
    
    setResults(filtered);
    setSelectedIndex(0);
  }, [query, parseNaturalQuery]);

  // Keyboard navigation
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex(i => Math.min(i + 1, results.length - 1));
          break;
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex(i => Math.max(i - 1, 0));
          break;
        case 'Enter':
          e.preventDefault();
          if (results[selectedIndex]) {
            results[selectedIndex].action();
            if (onCommand) onCommand(results[selectedIndex]);
            onClose();
          }
          break;
        case 'Escape':
          e.preventDefault();
          onClose();
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, results, selectedIndex, onClose, onCommand]);

  // Focus input when opened
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
      setQuery('');
    }
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-32">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/80" onClick={onClose} />
      
      {/* Command Palette */}
      <div className="relative w-full max-w-2xl bg-gray-950 border border-green-500/50 shadow-2xl">
        {/* Input */}
        <div className="flex items-center border-b border-gray-800">
          <span className="pl-4 text-green-500">&gt;</span>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Type a command or search (e.g., 'errors last 5m', 'slow queries', 'service:payment')"
            className="flex-1 px-4 py-3 bg-transparent text-white outline-none placeholder-gray-500 font-mono text-sm"
          />
          <kbd className="mr-4 px-2 py-1 text-xs bg-gray-800 text-gray-400 rounded">ESC</kbd>
        </div>
        
        {/* Results */}
        <div className="max-h-96 overflow-y-auto">
          {results.length === 0 ? (
            <div className="p-4 text-gray-500 text-sm">
              No commands found. Try natural language like "show errors in payment service"
            </div>
          ) : (
            results.map((cmd, index) => (
              <div
                key={cmd.id}
                className={`flex items-center justify-between px-4 py-2 cursor-pointer transition-colors ${
                  index === selectedIndex
                    ? 'bg-green-500/10 text-white border-l-2 border-green-500'
                    : 'text-gray-400 hover:bg-gray-900 hover:text-white'
                }`}
                onClick={() => {
                  cmd.action();
                  if (onCommand) onCommand(cmd);
                  onClose();
                }}
                onMouseEnter={() => setSelectedIndex(index)}
              >
                <div className="flex items-center gap-3">
                  {cmd.icon && <span className="text-lg">{cmd.icon}</span>}
                  <div>
                    <div className="font-mono text-sm">{cmd.label}</div>
                    <div className="text-xs text-gray-600">{cmd.category}</div>
                  </div>
                </div>
                {cmd.shortcut && (
                  <kbd className="px-2 py-1 text-xs bg-gray-800 text-gray-400 rounded">
                    {cmd.shortcut}
                  </kbd>
                )}
              </div>
            ))
          )}
        </div>
        
        {/* Footer hints */}
        <div className="border-t border-gray-800 px-4 py-2 flex items-center justify-between text-xs text-gray-500">
          <div className="flex gap-4">
            <span>↑↓ Navigate</span>
            <span>↵ Execute</span>
            <span>ESC Close</span>
          </div>
          <div>
            {results.length} commands
          </div>
        </div>
      </div>
    </div>
  );
};

export default CommandPalette;