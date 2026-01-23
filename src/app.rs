use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};

pub struct App {
    pub prd_name: String,
    pub remaining_tasks: usize,
    pub completed_tasks: usize,
    pub loop_count: u64,
    pub should_quit: bool,
    pub status_message: String,
    // Store all iteration logs
    pub iteration_logs: Vec<String>,
    pub current_log_index: usize,
    pub log_scroll_offset: usize,
    pub log_scroll_state: ScrollbarState,
}

impl App {
    pub fn new(prd_name: &str, remaining: usize, completed: usize) -> Self {
        Self {
            prd_name: prd_name.to_string(),
            remaining_tasks: remaining,
            completed_tasks: completed,
            loop_count: 0,
            should_quit: false,
            status_message: String::from("Initialising..."),
            iteration_logs: Vec::new(),
            current_log_index: 0,
            log_scroll_offset: 0,
            log_scroll_state: ScrollbarState::default(),
        }
    }

    /// Get the current log being viewed, or empty string if none
    fn current_log(&self) -> &str {
        self.iteration_logs
            .get(self.current_log_index)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let [top_area, log_area, footer_area] = Layout::vertical([
            Constraint::Length(7),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        self.render_top_panel(frame, top_area);
        self.render_log_panel(frame, log_area);
        self.render_footer(frame, footer_area);
    }

    fn render_top_panel(&self, frame: &mut Frame, area: Rect) {
        let border_color = Color::Green;
        let border_type = BorderType::Plain;

        let total_tasks = self.completed_tasks + self.remaining_tasks;
        let progress_str = format!("{}/{}", self.completed_tasks, total_tasks);
        let loop_str = format!("#{}", self.loop_count);

        let lines = vec![
            Line::from(vec![
                Span::styled("PRD: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&self.prd_name, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Progress: ", Style::default().fg(Color::White)),
                Span::styled(progress_str, Style::default().fg(Color::Yellow)),
                Span::styled(" tasks complete", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Loop: ", Style::default().fg(Color::White)),
                Span::styled(loop_str, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::White)),
                Span::styled(&self.status_message, Style::default().fg(Color::Gray)),
            ]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(Style::default().fg(border_color))
            .title(" Ralph's 'Special' Agent Loop ")
            .title_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::horizontal(1));

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_log_panel(&mut self, frame: &mut Frame, area: Rect) {
        let border_color = Color::Blue;
        let border_type = BorderType::Double;

        let current = self.current_log();
        // Compute content height from source to avoid borrow conflicts
        let content_height = if current.is_empty() {
            1
        } else {
            current.lines().count()
        };
        let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

        // Update scroll state before borrowing self for styled_lines
        self.log_scroll_state = ScrollbarState::default()
            .content_length(content_height)
            .viewport_content_length(visible_height)
            .position(self.log_scroll_offset);

        let styled_lines = self.parse_markdown_output();

        let log_title = if self.iteration_logs.is_empty() {
            " Iteration Log (waiting...) ".to_string()
        } else {
            format!(
                " Iteration Log [{}/{}] ",
                self.current_log_index + 1,
                self.iteration_logs.len()
            )
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(Style::default().fg(border_color))
            .title(log_title)
            .title_style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::horizontal(1));

        let paragraph = Paragraph::new(Text::from(styled_lines))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.log_scroll_offset as u16, 0));

        frame.render_widget(paragraph, area);

        // Render scrollbar
        if content_height > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("^"))
                .end_symbol(Some("v"));

            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut self.log_scroll_state,
            );
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let mode = if self.should_quit {
            "Quitting"
        } else {
            "Running"
        };

        let footer_text = Line::from(vec![
            Span::styled(" ralph v0.1.0 ", Style::default().fg(Color::Cyan)),
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled("Mode: ", Style::default().fg(Color::White)),
            Span::styled(mode, Style::default().fg(Color::Yellow)),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("<←/→>", Style::default().fg(Color::Green)),
            Span::styled(" logs  ", Style::default().fg(Color::Gray)),
            Span::styled("<↑/↓>", Style::default().fg(Color::Green)),
            Span::styled(" scroll  ", Style::default().fg(Color::Gray)),
            Span::styled("<q>", Style::default().fg(Color::Green)),
            Span::styled(" quit  ", Style::default().fg(Color::Gray)),
            Span::styled("<r>", Style::default().fg(Color::Green)),
            Span::styled(" resume", Style::default().fg(Color::Gray)),
        ]);

        let paragraph = Paragraph::new(footer_text).style(Style::default().bg(Color::DarkGray));

        frame.render_widget(paragraph, area);
    }

    fn parse_markdown_output(&self) -> Vec<Line<'_>> {
        let current = self.current_log();
        if current.is_empty() {
            return vec![Line::from(Span::styled(
                "Waiting for output...",
                Style::default().fg(Color::DarkGray),
            ))];
        }

        current
            .lines()
            .map(|line| {
                if line.starts_with("### ") {
                    // Header: cyan bold
                    Line::from(Span::styled(
                        line.strip_prefix("### ").unwrap_or(line),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if line.starts_with("## ") {
                    // H2: cyan bold
                    Line::from(Span::styled(
                        line.strip_prefix("## ").unwrap_or(line),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if line.starts_with("# ") {
                    // H1: cyan bold underline
                    Line::from(Span::styled(
                        line.strip_prefix("# ").unwrap_or(line),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    ))
                } else if line.trim_start().starts_with("* ") || line.trim_start().starts_with("- ")
                {
                    // Bullet point
                    let indent = line.len() - line.trim_start().len();
                    let content = line
                        .trim_start()
                        .strip_prefix("* ")
                        .or_else(|| line.trim_start().strip_prefix("- "))
                        .unwrap_or(line);

                    let bullet_color = if indent > 0 {
                        Color::Gray
                    } else {
                        Color::Yellow
                    };
                    let bullet_char = if indent > 0 { "  -" } else { "*" };

                    Line::from(vec![
                        Span::styled(" ".repeat(indent), Style::default()),
                        Span::styled(
                            format!("{} ", bullet_char),
                            Style::default().fg(bullet_color),
                        ),
                        Span::styled(content, Style::default().fg(Color::White)),
                    ])
                } else if line.contains('`') {
                    // Line with inline code - parse backticks
                    self.parse_inline_code(line)
                } else {
                    // Regular line
                    Line::from(Span::styled(line, Style::default().fg(Color::White)))
                }
            })
            .collect()
    }

    fn parse_inline_code(&self, line: &str) -> Line<'_> {
        let mut spans = Vec::new();
        let mut in_code = false;
        let mut current = String::new();

        for ch in line.chars() {
            if ch == '`' {
                if !current.is_empty() {
                    let style = if in_code {
                        Style::default().fg(Color::Magenta).bg(Color::Black)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    spans.push(Span::styled(current.clone(), style));
                    current.clear();
                }
                in_code = !in_code;
            } else {
                current.push(ch);
            }
        }

        // Handle remaining text
        if !current.is_empty() {
            let style = if in_code {
                Style::default().fg(Color::Magenta).bg(Color::Black)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(current, style));
        }

        Line::from(spans)
    }

    pub fn prev_log(&mut self) {
        if self.current_log_index > 0 {
            self.current_log_index -= 1;
            self.log_scroll_offset = 0;
        }
    }

    pub fn next_log(&mut self) {
        if self.current_log_index + 1 < self.iteration_logs.len() {
            self.current_log_index += 1;
            self.log_scroll_offset = 0;
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let content_height = self.current_log().lines().count();
        self.log_scroll_offset = self
            .log_scroll_offset
            .saturating_add(amount)
            .min(content_height);
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
    }

    pub fn increment_loop(&mut self) {
        self.loop_count += 1;
    }

    pub fn reload_progress(&mut self, remaining: usize, completed: usize) {
        self.remaining_tasks = remaining;
        self.completed_tasks = completed;
    }

    /// Add a new iteration log and switch to viewing it
    pub fn push_log(&mut self, output: String) {
        self.iteration_logs.push(output);
        self.current_log_index = self.iteration_logs.len() - 1;
        self.log_scroll_offset = 0;
    }

    /// Get the latest log content (for exit clause checking)
    pub fn latest_log(&self) -> Option<&str> {
        self.iteration_logs.last().map(|s| s.as_str())
    }
}
