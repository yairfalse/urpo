//! Ultra-fast input handling for sub-millisecond TUI response
//!
//! PERFORMANCE TARGET: <1ms keypress to screen update
//! Optimized for real-time observability workflows

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::{Duration, Instant};
use crate::core::Result;

/// Ultra-fast input command for zero-allocation processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastCommand {
    // Navigation (vim-style for speed)
    Up,
    Down,
    Left, 
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    
    // View switching (single key for speed)
    Services,       // 's'
    Traces,         // 't' 
    Logs,           // 'l'
    Metrics,        // 'm'
    Graph,          // 'g'
    
    // Actions
    Search,         // '/'
    Filter,         // 'f'
    Refresh,        // 'r'
    Export,         // 'e'
    
    // System
    Quit,           // 'q'
    Help,           // '?'
    
    // Special
    None,
}

/// Ultra-fast input processor with zero allocations in hot path
pub struct UltraFastInput {
    /// Last keypress timestamp for latency measurement
    last_keypress: Instant,
    /// Current command being processed
    current_command: FastCommand,
    /// Help overlay active
    help_active: bool,
    /// Command latency tracking (for optimization)
    avg_latency_ns: u64,
    /// Sample count for rolling average
    sample_count: u64,
}

impl UltraFastInput {
    /// Create new ultra-fast input handler
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            last_keypress: Instant::now(),
            current_command: FastCommand::None,
            help_active: false,
            avg_latency_ns: 0,
            sample_count: 0,
        }
    }
    
    /// Poll for input with zero-allocation hot path
    /// 
    /// PERFORMANCE: Target <100μs for this function
    #[inline(always)]
    pub fn poll_input(&mut self, timeout: Duration) -> Result<Option<FastCommand>> {
        // Fast path: check if event is available without blocking
        if !event::poll(timeout)? {
            return Ok(None);
        }
        
        let start = Instant::now();
        
        if let Ok(Event::Key(key)) = event::read() {
            self.last_keypress = start;
            
            let command = self.key_to_command_zero_copy(key);
            self.current_command = command;
            
            // Track latency for optimization (exponential moving average)
            let latency_ns = start.elapsed().as_nanos() as u64;
            if self.sample_count == 0 {
                self.avg_latency_ns = latency_ns;
            } else {
                // Alpha = 0.1 for smooth averaging
                self.avg_latency_ns = (self.avg_latency_ns * 9 + latency_ns) / 10;
            }
            self.sample_count += 1;
            
            // Log if we exceed 100μs target
            if latency_ns > 100_000 {
                tracing::warn!(
                    "Input processing exceeded 100μs: {}μs", 
                    latency_ns / 1000
                );
            }
            
            return Ok(Some(command));
        }
        
        Ok(None)
    }
    
    /// Convert keypress to command with zero allocations
    /// 
    /// PERFORMANCE: This function must be <10μs
    #[inline(always)]
    fn key_to_command_zero_copy(&mut self, key: KeyEvent) -> FastCommand {
        // Handle modifiers first (fastest path)
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => FastCommand::Quit,
                KeyCode::Char('r') => FastCommand::Refresh,
                _ => FastCommand::None,
            };
        }
        
        // Handle special keys
        match key.code {
            // Navigation (optimized order by frequency)
            KeyCode::Char('j') | KeyCode::Down => FastCommand::Down,
            KeyCode::Char('k') | KeyCode::Up => FastCommand::Up,
            KeyCode::Char('h') | KeyCode::Left => FastCommand::Left,
            KeyCode::Char('l') | KeyCode::Right => FastCommand::Right,
            KeyCode::PageDown | KeyCode::Char('d') => FastCommand::PageDown,
            KeyCode::PageUp | KeyCode::Char('u') => FastCommand::PageUp,
            KeyCode::Home | KeyCode::Char('g') => FastCommand::Home,
            KeyCode::End | KeyCode::Char('G') => FastCommand::End,
            
            // View switching (single key for max speed)
            KeyCode::Char('s') => FastCommand::Services,
            KeyCode::Char('t') => FastCommand::Traces,
            KeyCode::Char('L') => FastCommand::Logs,  // Capital L to avoid conflict
            KeyCode::Char('m') => FastCommand::Metrics,
            KeyCode::Char('v') => FastCommand::Graph,  // 'v' for view
            
            // Actions
            KeyCode::Char('/') => FastCommand::Search,
            KeyCode::Char('f') => FastCommand::Filter,
            KeyCode::Char('r') => FastCommand::Refresh,
            KeyCode::Char('e') => FastCommand::Export,
            
            // System
            KeyCode::Char('q') | KeyCode::Esc => FastCommand::Quit,
            KeyCode::Char('?') => {
                self.help_active = !self.help_active;
                FastCommand::Help
            },
            
            _ => FastCommand::None,
        }
    }
    
    /// Get current command without polling
    #[inline(always)]
    pub fn current_command(&self) -> FastCommand {
        self.current_command
    }
    
    /// Clear current command
    #[inline(always)]
    pub fn clear_command(&mut self) {
        self.current_command = FastCommand::None;
    }
    
    /// Get input latency metrics
    #[inline(always)]
    pub fn get_latency_metrics(&self) -> (u64, u64) {
        (self.avg_latency_ns, self.sample_count)
    }
    
    /// Render help overlay if active
    pub fn render_help_overlay(&self, f: &mut Frame, area: Rect) {
        if !self.help_active {
            return;
        }
        
        // Center the help overlay
        let help_area = centered_rect(60, 80, area);
        
        // Clear the background
        f.render_widget(Clear, help_area);
        
        let help_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" URPO Ultra-Fast Controls ", Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD))
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("NAVIGATION", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            ]),
            Line::from(vec![
                Span::styled("  j/↓  ", Style::default().fg(Color::Green)),
                Span::raw("Down     "),
                Span::styled("  k/↑  ", Style::default().fg(Color::Green)),
                Span::raw("Up")
            ]),
            Line::from(vec![
                Span::styled("  h/←  ", Style::default().fg(Color::Green)),
                Span::raw("Left     "),
                Span::styled("  l/→  ", Style::default().fg(Color::Green)),
                Span::raw("Right")
            ]),
            Line::from(vec![
                Span::styled("  d/PgDn ", Style::default().fg(Color::Green)),
                Span::raw("Page Down  "),
                Span::styled("  u/PgUp ", Style::default().fg(Color::Green)),
                Span::raw("Page Up")
            ]),
            Line::from(vec![
                Span::styled("  g/Home ", Style::default().fg(Color::Green)),
                Span::raw("Top      "),
                Span::styled("  G/End  ", Style::default().fg(Color::Green)),
                Span::raw("Bottom")
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("VIEWS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            ]),
            Line::from(vec![
                Span::styled("  s  ", Style::default().fg(Color::Yellow)),
                Span::raw("Services   "),
                Span::styled("  t  ", Style::default().fg(Color::Yellow)),
                Span::raw("Traces")
            ]),
            Line::from(vec![
                Span::styled("  L  ", Style::default().fg(Color::Yellow)),
                Span::raw("Logs       "),
                Span::styled("  m  ", Style::default().fg(Color::Yellow)),
                Span::raw("Metrics")
            ]),
            Line::from(vec![
                Span::styled("  v  ", Style::default().fg(Color::Yellow)),
                Span::raw("Graph View")
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("ACTIONS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            ]),
            Line::from(vec![
                Span::styled("  /  ", Style::default().fg(Color::Magenta)),
                Span::raw("Search     "),
                Span::styled("  f  ", Style::default().fg(Color::Magenta)),
                Span::raw("Filter")
            ]),
            Line::from(vec![
                Span::styled("  r  ", Style::default().fg(Color::Magenta)),
                Span::raw("Refresh    "),
                Span::styled("  e  ", Style::default().fg(Color::Magenta)),
                Span::raw("Export")
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("SYSTEM", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            ]),
            Line::from(vec![
                Span::styled("  q/Esc ", Style::default().fg(Color::Red)),
                Span::raw("Quit      "),
                Span::styled("  ?  ", Style::default().fg(Color::Blue)),
                Span::raw("Help")
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("PERFORMANCE", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            ]),
            Line::from(vec![
                Span::raw("Input Latency: "),
                Span::styled(
                    format!("{}μs", self.avg_latency_ns / 1000),
                    if self.avg_latency_ns < 100_000 {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    }
                )
            ]),
            Line::from(vec![
                Span::raw("Samples: "),
                Span::styled(
                    format!("{}", self.sample_count),
                    Style::default().fg(Color::Cyan)
                )
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ? to close help", Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC))
            ]),
            Line::from(""),
        ];
        
        let help_block = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title(" Help - Ultra-Fast Controls ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
            )
            .wrap(ratatui::widgets::Wrap { trim: true })
            .alignment(Alignment::Left);
        
        f.render_widget(help_block, help_area);
    }
}

impl Default for UltraFastInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_key_to_command_navigation() {
        let mut input = UltraFastInput::new();
        
        // Test vim navigation
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            FastCommand::Down
        );
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
            FastCommand::Up
        );
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
            FastCommand::Left
        );
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
            FastCommand::Right
        );
    }

    #[test]
    fn test_key_to_command_views() {
        let mut input = UltraFastInput::new();
        
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
            FastCommand::Services
        );
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
            FastCommand::Traces
        );
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE)),
            FastCommand::Metrics
        );
    }

    #[test]
    fn test_key_to_command_system() {
        let mut input = UltraFastInput::new();
        
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            FastCommand::Quit
        );
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            FastCommand::Quit
        );
    }

    #[test]
    fn test_ctrl_commands() {
        let mut input = UltraFastInput::new();
        
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            FastCommand::Quit
        );
        assert_eq!(
            input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            FastCommand::Refresh
        );
    }

    #[test]
    fn test_help_toggle() {
        let mut input = UltraFastInput::new();
        
        assert!(!input.help_active);
        
        input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(input.help_active);
        
        input.key_to_command_zero_copy(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(!input.help_active);
    }
}