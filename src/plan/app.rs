use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use super::phases::PlanPhase;
use super::protocol::{Answer, PlanResponse, Question};

/// Input mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Navigating questions/options
    Normal,
    /// Typing freeform input
    Editing,
}

/// TUI state for plan mode
pub struct PlanApp {
    /// Current phase from Claude's response
    pub phase: PlanPhase,

    /// Status message to display
    pub status: String,

    /// Whether we're in the initial idea input phase (before Claude)
    pub awaiting_idea: bool,

    /// Whether we're in a processing state (between answer submission and Claude response)
    pub processing: bool,

    /// Message to display during processing
    pub processing_message: String,

    /// Spinner animation frame (0-7 for braille spinner)
    pub spinner_frame: u8,

    /// Number of answers submitted (captured when entering processing state)
    pub submitted_count: usize,

    /// Total questions count (captured when entering processing state)
    pub submitted_total: usize,

    /// The user's idea/description input
    pub idea_input: String,

    /// Cursor position for idea input
    pub idea_cursor: usize,

    /// Questions to display (when in asking phase)
    pub questions: Vec<Question>,

    /// Index of currently selected question
    pub current_question: usize,

    /// Selected option index for current question
    pub selected_option: Option<usize>,

    /// List state for option selection
    pub option_list_state: ListState,

    /// Freeform input text
    pub freeform_input: String,

    /// Cursor position in freeform input
    pub cursor_position: usize,

    /// Current input mode
    pub input_mode: InputMode,

    /// Collected answers
    pub answers: Vec<Answer>,

    /// Turn count
    pub turn_count: u32,

    /// Should quit the application
    pub should_quit: bool,

    /// Should submit all answers and continue to next Claude turn
    pub should_submit: bool,

    /// Log of Claude responses for viewing
    pub response_logs: Vec<String>,

    /// Current log index being viewed
    pub current_log_index: usize,

    /// Scroll offset for log viewing
    pub log_scroll_offset: usize,

    /// Scrollbar state for log viewing
    pub log_scroll_state: ScrollbarState,
}

impl PlanApp {
    pub fn new() -> Self {
        Self {
            phase: PlanPhase::Exploring,
            status: String::from("Starting..."),
            awaiting_idea: false,
            processing: false,
            processing_message: String::new(),
            spinner_frame: 0,
            submitted_count: 0,
            submitted_total: 0,
            idea_input: String::new(),
            idea_cursor: 0,
            questions: Vec::new(),
            current_question: 0,
            selected_option: None,
            option_list_state: ListState::default(),
            freeform_input: String::new(),
            cursor_position: 0,
            input_mode: InputMode::Normal,
            answers: Vec::new(),
            turn_count: 0,
            should_quit: false,
            should_submit: false,
            response_logs: Vec::new(),
            current_log_index: 0,
            log_scroll_offset: 0,
            log_scroll_state: ScrollbarState::default(),
        }
    }

    /// Update TUI state from a Claude response
    pub fn update_from_response(&mut self, response: &PlanResponse) {
        self.phase = response.phase;

        if let Some(ref status) = response.status {
            self.status = status.clone();
        }

        if let Some(ref questions) = response.questions {
            self.questions = questions.clone();
            self.current_question = 0;
            self.selected_option = None;
            self.option_list_state.select(Some(0));
            self.freeform_input.clear();
            self.cursor_position = 0;
        }

        self.turn_count += 1;
    }

    /// Set questions to display
    pub fn set_questions(&mut self, questions: Vec<Question>) {
        self.questions = questions;
        self.current_question = 0;
        self.selected_option = None;
        self.option_list_state.select(Some(0));
        self.freeform_input.clear();
        self.cursor_position = 0;
    }

    /// Get the current question being displayed
    pub fn current_question(&self) -> Option<&Question> {
        self.questions.get(self.current_question)
    }

    /// Move to next question
    pub fn next_question(&mut self) {
        if self.current_question + 1 < self.questions.len() {
            self.current_question += 1;
            self.selected_option = None;
            self.option_list_state.select(Some(0));
            self.freeform_input.clear();
            self.cursor_position = 0;
        }
    }

    /// Move to previous question
    pub fn prev_question(&mut self) {
        if self.current_question > 0 {
            self.current_question -= 1;
            self.selected_option = None;
            self.option_list_state.select(Some(0));
            self.freeform_input.clear();
            self.cursor_position = 0;
        }
    }

    /// Select next option in list
    pub fn next_option(&mut self) {
        if let Some(q) = self.current_question()
            && let Some(ref opts) = q.options
        {
            let i = self.option_list_state.selected().unwrap_or(0);
            let next = if i + 1 >= opts.len() { 0 } else { i + 1 };
            self.option_list_state.select(Some(next));
        }
    }

    /// Select previous option in list
    pub fn prev_option(&mut self) {
        if let Some(q) = self.current_question()
            && let Some(ref opts) = q.options
        {
            let i = self.option_list_state.selected().unwrap_or(0);
            let prev = if i == 0 { opts.len() - 1 } else { i - 1 };
            self.option_list_state.select(Some(prev));
        }
    }

    /// Submit answer for current question (replaces existing answer if any)
    pub fn submit_answer(&mut self) {
        if let Some(q) = self.questions.get(self.current_question).cloned() {
            let value = if self.input_mode == InputMode::Editing || q.options.is_none() {
                // Use freeform input
                self.freeform_input.clone()
            } else if let Some(ref opts) = q.options {
                // Use selected option
                let idx = self.option_list_state.selected().unwrap_or(0);
                opts.get(idx).map(|o| o.key.clone()).unwrap_or_default()
            } else {
                String::new()
            };

            if !value.is_empty() {
                // Replace existing answer for this question (don't add duplicates)
                if let Some(existing) = self.answers.iter_mut().find(|a| a.question_id == q.id) {
                    existing.value = value;
                } else {
                    self.answers.push(Answer {
                        question_id: q.id.clone(),
                        value,
                    });
                }
            }
        }
    }

    /// Enter editing mode for freeform input
    pub fn enter_editing(&mut self) {
        self.input_mode = InputMode::Editing;
    }

    /// Exit editing mode
    pub fn exit_editing(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Handle character input in editing mode
    pub fn enter_char(&mut self, c: char) {
        self.freeform_input.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Handle backspace in editing mode
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.freeform_input.remove(self.cursor_position);
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.freeform_input.len() {
            self.cursor_position += 1;
        }
    }

    /// Take collected answers (consumes them)
    pub fn take_answers(&mut self) -> Vec<Answer> {
        std::mem::take(&mut self.answers)
    }

    /// Check if all questions have been answered
    pub fn all_answered(&self) -> bool {
        if self.questions.is_empty() {
            return false;
        }
        // Check that every question has an answer
        self.questions
            .iter()
            .all(|q| self.answers.iter().any(|a| a.question_id == q.id))
    }

    /// Get count of answered questions (unique question IDs)
    pub fn answered_count(&self) -> usize {
        self.questions
            .iter()
            .filter(|q| self.answers.iter().any(|a| a.question_id == q.id))
            .count()
    }

    /// Reset submit flag
    pub fn reset_submit(&mut self) {
        self.should_submit = false;
    }

    /// Set processing state with a message
    /// When activating, captures the current answer/question counts
    pub fn set_processing(&mut self, active: bool, message: &str) {
        self.processing = active;
        self.processing_message = message.to_string();
        if active {
            self.spinner_frame = 0;
            // Capture counts at the moment of submission
            self.submitted_count = self.answered_count();
            self.submitted_total = self.questions.len();
        }
    }

    /// Advance spinner animation frame
    pub fn advance_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 8;
    }

    /// Get current spinner character (braille spinner)
    fn spinner_char(&self) -> char {
        const SPINNER_FRAMES: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
        SPINNER_FRAMES[self.spinner_frame as usize]
    }

    /// Push a log entry
    pub fn push_log(&mut self, log: String) {
        self.response_logs.push(log);
        self.current_log_index = self.response_logs.len().saturating_sub(1);
        self.log_scroll_offset = 0;
    }

    /// Get current log
    fn current_log(&self) -> &str {
        self.response_logs
            .get(self.current_log_index)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Draw the TUI
    pub fn draw(&mut self, frame: &mut Frame) {
        // Show idea input screen if awaiting initial idea
        if self.awaiting_idea {
            self.render_idea_input(frame, frame.area());
            return;
        }

        // Show processing screen if in processing state
        if self.processing {
            self.render_processing(frame, frame.area());
            return;
        }

        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        self.render_header(frame, header_area);

        match self.phase {
            PlanPhase::Asking => self.render_questions(frame, main_area),
            _ => self.render_status_panel(frame, main_area),
        }

        self.render_footer(frame, footer_area);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let phase_indicators: Vec<Span> = [
            PlanPhase::Exploring,
            PlanPhase::Asking,
            PlanPhase::Working,
            PlanPhase::Complete,
        ]
        .iter()
        .map(|p| {
            let symbol = if *p == self.phase { "●" } else { "○" };
            let color = if *p == self.phase {
                Color::Green
            } else {
                Color::DarkGray
            };
            Span::styled(format!(" {} ", symbol), Style::default().fg(color))
        })
        .collect();

        // Build progress indicator for asking phase (or processing state)
        let progress_span = if self.processing {
            // Use captured counts during processing (answers may be consumed)
            let answered = self.submitted_count;
            let total = self.submitted_total;
            vec![
                Span::styled(" | Submitted: ", Style::default().fg(Color::Gray)),
                Span::styled(format!("{}/{}", answered, total), Style::default().fg(Color::Green)),
            ]
        } else if self.phase == PlanPhase::Asking && !self.questions.is_empty() {
            let answered = self.answered_count();
            let total = self.questions.len();
            let color = if self.all_answered() {
                Color::Green
            } else {
                Color::Yellow
            };
            vec![
                Span::styled(" | Answered: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{}/{}", answered, total),
                    Style::default().fg(color),
                ),
            ]
        } else {
            vec![]
        };

        let mut header_line = vec![
            Span::styled("Ralph Plan", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" | Turn: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("#{}", self.turn_count),
                Style::default().fg(Color::Cyan),
            ),
        ];
        header_line.extend(progress_span);

        let lines = vec![
            Line::from(header_line),
            Line::from(vec![
                Span::styled("Phase: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    self.phase.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(phase_indicators),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green))
            .title(" Ralph PRD Generator ")
            .title_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::horizontal(1));

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_questions(&mut self, frame: &mut Frame, area: Rect) {
        if self.questions.is_empty() {
            self.render_status_panel(frame, area);
            return;
        }

        // Render current question
        if let Some(q) = self.questions.get(self.current_question) {
            let has_options = q.options.is_some();
            let allows_freeform = q.allow_freeform || q.options.is_none();

            // Dynamic layout: collapse empty space, give freeform prominence when needed
            let (question_area, options_area, input_area) = if has_options && allows_freeform {
                // Both options AND freeform: compact layout with visible input
                let option_count = q.options.as_ref().map(|o| o.len()).unwrap_or(0);
                let options_height = (option_count as u16 + 3).min(12); // +3 for borders/title, max 12
                let [q_area, o_area, i_area, _spacer] = Layout::vertical([
                    Constraint::Length(6),              // Question
                    Constraint::Length(options_height), // Options (sized to content)
                    Constraint::Length(5),              // Freeform input (more prominent)
                    Constraint::Fill(1),                // Absorb remaining space
                ])
                .areas(area);
                (q_area, o_area, i_area)
            } else if has_options {
                // Only options, no freeform
                let [q_area, o_area, i_area] = Layout::vertical([
                    Constraint::Length(6),
                    Constraint::Fill(1),
                    Constraint::Length(0), // No input area
                ])
                .areas(area);
                (q_area, o_area, i_area)
            } else {
                // Only freeform, no options - give input more space
                let [q_area, o_area, i_area, _spacer] = Layout::vertical([
                    Constraint::Length(6),
                    Constraint::Length(7), // Hint area
                    Constraint::Length(5), // Input area
                    Constraint::Fill(1),   // Absorb remaining
                ])
                .areas(area);
                (q_area, o_area, i_area)
            };

            // === Question Block ===
            let question_lines = vec![
                Line::from(vec![
                    Span::styled(
                        format!("[{}] ", q.category.to_uppercase()),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!(
                            "Question {}/{}",
                            self.current_question + 1,
                            self.questions.len()
                        ),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    &q.text,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    q.context.as_deref().unwrap_or(""),
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let question_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::Blue))
                .padding(Padding::horizontal(1));

            let question_widget = Paragraph::new(question_lines)
                .block(question_block)
                .wrap(Wrap { trim: false });

            frame.render_widget(question_widget, question_area);

            // === Options Block ===
            if let Some(ref opts) = q.options {
                let items: Vec<ListItem> = opts
                    .iter()
                    .map(|opt| {
                        let content = if let Some(ref desc) = opt.description {
                            format!("{}) {} - {}", opt.key, opt.label, desc)
                        } else {
                            format!("{}) {}", opt.key, opt.label)
                        };
                        ListItem::new(content)
                    })
                    .collect();

                let options_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Options (↑↓ to select, Enter to confirm) ")
                    .padding(Padding::horizontal(1));

                let options_list = List::new(items)
                    .block(options_block)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");

                frame.render_stateful_widget(
                    options_list,
                    options_area,
                    &mut self.option_list_state,
                );
            } else {
                // No predefined options - show prominent hint for freeform input
                let hint_lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("  ╭", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            "───────────────────────────────────",
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled("╮", Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::styled("  │  ", Style::default().fg(Color::Yellow)),
                        Span::styled("PRESS ", Style::default().fg(Color::White)),
                        Span::styled(
                            " i ",
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" TO TYPE YOUR RESPONSE", Style::default().fg(Color::White)),
                        Span::styled("   │", Style::default().fg(Color::Yellow)),
                    ]),
                    Line::from(vec![
                        Span::styled("  ╰", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            "───────────────────────────────────",
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled("╯", Style::default().fg(Color::Yellow)),
                    ]),
                ];

                let hint_widget =
                    Paragraph::new(hint_lines).alignment(ratatui::layout::Alignment::Center);

                frame.render_widget(hint_widget, options_area);
            }

            // === Freeform Input Block ===
            if allows_freeform {
                let is_editing = self.input_mode == InputMode::Editing;

                // Make it MORE prominent when freeform is available
                let (border_style, title_style, bg_hint) = if is_editing {
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                        "",
                    )
                } else if has_options {
                    // Options exist but freeform allowed - highlight the input
                    (
                        Style::default().fg(Color::Cyan),
                        Style::default().fg(Color::Cyan),
                        " ← press 'i' ",
                    )
                } else {
                    // No options - freeform is the only way
                    (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::Yellow),
                        "",
                    )
                };

                let title = if is_editing {
                    " ✎ TYPING... (Esc to finish, Enter to submit) ".to_string()
                } else if has_options {
                    format!(" Or type custom answer{} ", bg_hint)
                } else {
                    format!(" Type your answer{} ", bg_hint)
                };

                let input_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(if is_editing {
                        BorderType::Double
                    } else {
                        BorderType::Plain
                    })
                    .border_style(border_style)
                    .title(Span::styled(title, title_style))
                    .padding(Padding::horizontal(1));

                // Show placeholder when empty and not editing
                let display_text = if self.freeform_input.is_empty() && !is_editing {
                    Span::styled(
                        "Press 'i' to start typing...",
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::styled(&self.freeform_input, Style::default().fg(Color::White))
                };

                let input_widget = Paragraph::new(Line::from(display_text)).block(input_block);

                frame.render_widget(input_widget, input_area);

                // Show cursor in editing mode
                if is_editing {
                    frame.set_cursor_position((
                        input_area.x + self.cursor_position as u16 + 2,
                        input_area.y + 1,
                    ));
                }
            }
        }
    }

    fn render_status_panel(&mut self, frame: &mut Frame, area: Rect) {
        // Compute content height without borrowing self
        let content_height = self
            .response_logs
            .get(self.current_log_index)
            .map(|s| if s.is_empty() { 1 } else { s.lines().count() })
            .unwrap_or(1);
        let visible_height = area.height.saturating_sub(2) as usize;

        self.log_scroll_state = ScrollbarState::default()
            .content_length(content_height)
            .viewport_content_length(visible_height)
            .position(self.log_scroll_offset);

        // Now we can borrow current_log for building lines
        let current = self.current_log();
        let lines: Vec<Line> = if current.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    self.status.clone(),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Waiting for Claude...",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else {
            current
                .lines()
                .map(|line| {
                    Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::White),
                    ))
                })
                .collect()
        };

        let title = match self.phase {
            PlanPhase::Exploring => " Exploring Codebase ",
            PlanPhase::Working => " Generating PRD ",
            PlanPhase::Complete => " PRD Complete! ",
            PlanPhase::Asking => " Questions ",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Blue))
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::horizontal(1));

        let paragraph = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.log_scroll_offset as u16, 0));

        frame.render_widget(paragraph, area);

        // Render scrollbar if needed
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

    fn render_processing(&self, frame: &mut Frame, area: Rect) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Render header (reuse existing)
        self.render_header(frame, header_area);

        // Processing panel with spinner and status
        // Use captured counts (answers may be consumed by take_answers())
        let spinner = self.spinner_char();

        let lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("         {} ", spinner),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &self.processing_message,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                format!(
                    "         Submitted {}/{} answers",
                    self.submitted_count, self.submitted_total
                ),
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Processing ")
            .title_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .padding(Padding::horizontal(1));

        let paragraph = Paragraph::new(Text::from(lines))
            .block(block)
            .alignment(ratatui::layout::Alignment::Left);

        frame.render_widget(paragraph, main_area);

        // Processing footer
        let footer_spans = vec![
            Span::styled(" ralph plan ", Style::default().fg(Color::Cyan)),
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled("<Ctrl+C>", Style::default().fg(Color::Green)),
            Span::styled(" cancel ", Style::default().fg(Color::Gray)),
        ];

        let footer =
            Paragraph::new(Line::from(footer_spans)).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(footer, footer_area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let keybinds = match self.phase {
            PlanPhase::Asking => {
                if self.input_mode == InputMode::Editing {
                    vec![
                        ("<Esc>", "finish typing"),
                        ("<Enter>", "next"),
                        ("<Backspace>", "delete"),
                    ]
                } else if self.all_answered() {
                    // All questions answered - show submit option prominently
                    vec![
                        ("<C-Enter>", "SUBMIT ALL"),
                        ("<↑↓>", "options"),
                        ("<Tab>", "review"),
                        ("<q>", "quit"),
                    ]
                } else {
                    vec![
                        ("<↑↓>", "options"),
                        ("<Tab>", "next Q"),
                        ("<i>", "type"),
                        ("<Enter>", "answer"),
                        ("<q>", "quit"),
                    ]
                }
            }
            _ => vec![("<q>", "quit"), ("<↑↓>", "scroll")],
        };

        let mut spans = vec![
            Span::styled(" ralph plan ", Style::default().fg(Color::Cyan)),
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
        ];

        for (key, action) in keybinds {
            spans.push(Span::styled(key, Style::default().fg(Color::Green)));
            spans.push(Span::styled(
                format!(" {} ", action),
                Style::default().fg(Color::Gray),
            ));
        }

        let footer = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(footer, area);
    }

    fn render_idea_input(&self, frame: &mut Frame, area: Rect) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // Header
        let header_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green))
            .title(" Ralph Plan ")
            .title_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            );

        let header = Paragraph::new(Line::from(vec![Span::styled(
            "Interactive PRD Generator",
            Style::default().fg(Color::Cyan),
        )]))
        .block(header_block)
        .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(header, header_area);

        // Main input area
        let [prompt_area, input_area] =
            Layout::vertical([Constraint::Length(5), Constraint::Fill(1)]).areas(main_area);

        // Prompt text
        let prompt_block = Block::default()
            .borders(Borders::NONE)
            .padding(Padding::new(2, 2, 1, 0));

        let prompt_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "What do you want to build?",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Describe your idea below. Claude will explore the codebase and generate a PRD.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let prompt = Paragraph::new(prompt_lines).block(prompt_block);
        frame.render_widget(prompt, prompt_area);

        // Input box
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Your Idea ")
            .title_style(Style::default().fg(Color::Yellow))
            .padding(Padding::horizontal(1));

        let input_text = if self.idea_input.is_empty() {
            Span::styled("Start typing...", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(&self.idea_input, Style::default().fg(Color::White))
        };

        let input = Paragraph::new(Line::from(input_text))
            .block(input_block)
            .wrap(Wrap { trim: false });

        frame.render_widget(input, input_area);

        // Position cursor
        if !self.idea_input.is_empty() || self.idea_cursor > 0 {
            frame.set_cursor_position((
                input_area.x + self.idea_cursor as u16 + 2,
                input_area.y + 1,
            ));
        } else {
            frame.set_cursor_position((input_area.x + 2, input_area.y + 1));
        }

        // Footer
        let footer_spans = vec![
            Span::styled(" ralph plan ", Style::default().fg(Color::Cyan)),
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled("<Enter>", Style::default().fg(Color::Green)),
            Span::styled(" Start ", Style::default().fg(Color::Gray)),
            Span::styled("<Esc>", Style::default().fg(Color::Green)),
            Span::styled(" Quit ", Style::default().fg(Color::Gray)),
        ];

        let footer =
            Paragraph::new(Line::from(footer_spans)).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(footer, footer_area);
    }

    /// Scroll up in log view
    pub fn scroll_up(&mut self, amount: usize) {
        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(amount);
    }

    /// Scroll down in log view
    pub fn scroll_down(&mut self, amount: usize) {
        let content_height = self.current_log().lines().count();
        self.log_scroll_offset = self
            .log_scroll_offset
            .saturating_add(amount)
            .min(content_height);
    }
}

impl Default for PlanApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::protocol::QuestionOption;

    fn create_test_question(id: &str, with_options: bool) -> Question {
        Question {
            id: id.to_string(),
            category: "scope".to_string(),
            text: format!("Question {id}?"),
            context: Some("Context".to_string()),
            options: if with_options {
                Some(vec![
                    QuestionOption {
                        key: "A".to_string(),
                        label: "Option A".to_string(),
                        description: None,
                    },
                    QuestionOption {
                        key: "B".to_string(),
                        label: "Option B".to_string(),
                        description: Some("With description".to_string()),
                    },
                ])
            } else {
                None
            },
            allow_freeform: true,
        }
    }

    #[test]
    fn new_app_initialization() {
        let app = PlanApp::new();
        assert_eq!(app.phase, PlanPhase::Exploring);
        assert_eq!(app.status, "Starting...");
        assert!(!app.awaiting_idea);
        assert!(app.idea_input.is_empty());
        assert!(app.questions.is_empty());
        assert_eq!(app.current_question, 0);
        assert!(app.answers.is_empty());
        assert_eq!(app.turn_count, 0);
        assert!(!app.should_quit);
        assert!(!app.should_submit);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn default_same_as_new() {
        let default_app = PlanApp::default();
        let new_app = PlanApp::new();
        assert_eq!(default_app.phase, new_app.phase);
        assert_eq!(default_app.status, new_app.status);
        assert_eq!(default_app.turn_count, new_app.turn_count);
    }

    #[test]
    fn update_from_response_changes_phase_and_status() {
        let mut app = PlanApp::new();
        let response = PlanResponse {
            phase: PlanPhase::Asking,
            status: Some("Need input".to_string()),
            questions: None,
            context: None,
            prd: None,
        };

        app.update_from_response(&response);
        assert_eq!(app.phase, PlanPhase::Asking);
        assert_eq!(app.status, "Need input");
        assert_eq!(app.turn_count, 1);
    }

    #[test]
    fn update_from_response_sets_questions() {
        let mut app = PlanApp::new();
        let response = PlanResponse {
            phase: PlanPhase::Asking,
            status: None,
            questions: Some(vec![
                create_test_question("q1", true),
                create_test_question("q2", false),
            ]),
            context: None,
            prd: None,
        };

        app.update_from_response(&response);
        assert_eq!(app.questions.len(), 2);
        assert_eq!(app.current_question, 0);
        assert!(app.freeform_input.is_empty());
    }

    #[test]
    fn next_question_navigation() {
        let mut app = PlanApp::new();
        app.set_questions(vec![
            create_test_question("q1", true),
            create_test_question("q2", true),
            create_test_question("q3", true),
        ]);

        assert_eq!(app.current_question, 0);
        app.next_question();
        assert_eq!(app.current_question, 1);
        app.next_question();
        assert_eq!(app.current_question, 2);
        // Can't go past last
        app.next_question();
        assert_eq!(app.current_question, 2);
    }

    #[test]
    fn prev_question_navigation() {
        let mut app = PlanApp::new();
        app.set_questions(vec![
            create_test_question("q1", true),
            create_test_question("q2", true),
        ]);
        app.current_question = 1;

        app.prev_question();
        assert_eq!(app.current_question, 0);
        // Can't go below 0
        app.prev_question();
        assert_eq!(app.current_question, 0);
    }

    #[test]
    fn question_navigation_resets_state() {
        let mut app = PlanApp::new();
        app.set_questions(vec![
            create_test_question("q1", true),
            create_test_question("q2", true),
        ]);

        app.freeform_input = "some text".to_string();
        app.cursor_position = 5;
        app.option_list_state.select(Some(1));

        app.next_question();
        assert!(app.freeform_input.is_empty());
        assert_eq!(app.cursor_position, 0);
        assert_eq!(app.option_list_state.selected(), Some(0));
    }

    #[test]
    fn next_option_cycles() {
        let mut app = PlanApp::new();
        app.set_questions(vec![create_test_question("q1", true)]);
        app.option_list_state.select(Some(0));

        app.next_option();
        assert_eq!(app.option_list_state.selected(), Some(1));

        app.next_option();
        assert_eq!(app.option_list_state.selected(), Some(0)); // Wraps around
    }

    #[test]
    fn prev_option_cycles() {
        let mut app = PlanApp::new();
        app.set_questions(vec![create_test_question("q1", true)]);
        app.option_list_state.select(Some(0));

        app.prev_option();
        assert_eq!(app.option_list_state.selected(), Some(1)); // Wraps to end

        app.prev_option();
        assert_eq!(app.option_list_state.selected(), Some(0));
    }

    #[test]
    fn submit_answer_from_option() {
        let mut app = PlanApp::new();
        app.set_questions(vec![create_test_question("q1", true)]);
        app.option_list_state.select(Some(1)); // Select option B
        app.input_mode = InputMode::Normal;

        app.submit_answer();
        assert_eq!(app.answers.len(), 1);
        assert_eq!(app.answers[0].question_id, "q1");
        assert_eq!(app.answers[0].value, "B");
    }

    #[test]
    fn submit_answer_from_freeform() {
        let mut app = PlanApp::new();
        app.set_questions(vec![create_test_question("q1", true)]);
        app.input_mode = InputMode::Editing;
        app.freeform_input = "Custom answer".to_string();

        app.submit_answer();
        assert_eq!(app.answers.len(), 1);
        assert_eq!(app.answers[0].value, "Custom answer");
    }

    #[test]
    fn submit_answer_no_options_uses_freeform() {
        let mut app = PlanApp::new();
        app.set_questions(vec![create_test_question("q1", false)]); // No options
        app.input_mode = InputMode::Normal;
        app.freeform_input = "Freeform only".to_string();

        app.submit_answer();
        assert_eq!(app.answers.len(), 1);
        assert_eq!(app.answers[0].value, "Freeform only");
    }

    #[test]
    fn submit_empty_answer_not_added() {
        let mut app = PlanApp::new();
        app.set_questions(vec![create_test_question("q1", false)]);
        app.freeform_input = String::new();

        app.submit_answer();
        assert!(app.answers.is_empty());
    }

    #[test]
    fn enter_exit_editing_mode() {
        let mut app = PlanApp::new();
        assert_eq!(app.input_mode, InputMode::Normal);

        app.enter_editing();
        assert_eq!(app.input_mode, InputMode::Editing);

        app.exit_editing();
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn enter_char_inserts_at_cursor() {
        let mut app = PlanApp::new();
        app.enter_editing();

        app.enter_char('H');
        app.enter_char('i');
        assert_eq!(app.freeform_input, "Hi");
        assert_eq!(app.cursor_position, 2);
    }

    #[test]
    fn enter_char_middle_of_string() {
        let mut app = PlanApp::new();
        app.freeform_input = "Hllo".to_string();
        app.cursor_position = 1;

        app.enter_char('e');
        assert_eq!(app.freeform_input, "Hello");
        assert_eq!(app.cursor_position, 2);
    }

    #[test]
    fn delete_char_removes_before_cursor() {
        let mut app = PlanApp::new();
        app.freeform_input = "Hello".to_string();
        app.cursor_position = 5;

        app.delete_char();
        assert_eq!(app.freeform_input, "Hell");
        assert_eq!(app.cursor_position, 4);
    }

    #[test]
    fn delete_char_at_start_does_nothing() {
        let mut app = PlanApp::new();
        app.freeform_input = "Hello".to_string();
        app.cursor_position = 0;

        app.delete_char();
        assert_eq!(app.freeform_input, "Hello");
        assert_eq!(app.cursor_position, 0);
    }

    #[test]
    fn move_cursor_left() {
        let mut app = PlanApp::new();
        app.freeform_input = "Hello".to_string();
        app.cursor_position = 3;

        app.move_cursor_left();
        assert_eq!(app.cursor_position, 2);

        app.cursor_position = 0;
        app.move_cursor_left();
        assert_eq!(app.cursor_position, 0); // Can't go below 0
    }

    #[test]
    fn move_cursor_right() {
        let mut app = PlanApp::new();
        app.freeform_input = "Hello".to_string();
        app.cursor_position = 3;

        app.move_cursor_right();
        assert_eq!(app.cursor_position, 4);

        app.cursor_position = 5;
        app.move_cursor_right();
        assert_eq!(app.cursor_position, 5); // Can't go past end
    }

    #[test]
    fn take_answers_consumes_and_clears() {
        let mut app = PlanApp::new();
        app.answers.push(Answer {
            question_id: "q1".to_string(),
            value: "A".to_string(),
        });
        app.answers.push(Answer {
            question_id: "q2".to_string(),
            value: "B".to_string(),
        });

        let taken = app.take_answers();
        assert_eq!(taken.len(), 2);
        assert!(app.answers.is_empty());
    }

    #[test]
    fn all_answered_check() {
        let mut app = PlanApp::new();
        app.set_questions(vec![
            create_test_question("q1", true),
            create_test_question("q2", true),
        ]);

        assert!(!app.all_answered());

        app.answers.push(Answer {
            question_id: "q1".to_string(),
            value: "A".to_string(),
        });
        assert!(!app.all_answered());

        app.answers.push(Answer {
            question_id: "q2".to_string(),
            value: "B".to_string(),
        });
        assert!(app.all_answered());
    }

    #[test]
    fn all_answered_false_when_no_questions() {
        let app = PlanApp::new();
        assert!(!app.all_answered()); // No questions means not all answered
    }

    #[test]
    fn answered_count() {
        let mut app = PlanApp::new();
        app.set_questions(vec![
            create_test_question("q1", true),
            create_test_question("q2", true),
        ]);
        assert_eq!(app.answered_count(), 0);

        app.answers.push(Answer {
            question_id: "q1".to_string(),
            value: "A".to_string(),
        });
        assert_eq!(app.answered_count(), 1);

        app.answers.push(Answer {
            question_id: "q2".to_string(),
            value: "B".to_string(),
        });
        assert_eq!(app.answered_count(), 2);

        // Adding duplicate answer for q1 should NOT increase count
        app.answers.push(Answer {
            question_id: "q1".to_string(),
            value: "C".to_string(),
        });
        assert_eq!(app.answered_count(), 2); // Still 2, not 3
    }

    #[test]
    fn current_question_returns_correct_question() {
        let mut app = PlanApp::new();
        app.set_questions(vec![
            create_test_question("q1", true),
            create_test_question("q2", true),
        ]);

        assert_eq!(app.current_question().unwrap().id, "q1");
        app.current_question = 1;
        assert_eq!(app.current_question().unwrap().id, "q2");
    }

    #[test]
    fn current_question_none_when_empty() {
        let app = PlanApp::new();
        assert!(app.current_question().is_none());
    }

    #[test]
    fn push_log_and_scroll() {
        let mut app = PlanApp::new();
        app.push_log("Log 1".to_string());
        assert_eq!(app.response_logs.len(), 1);
        assert_eq!(app.current_log_index, 0);

        app.push_log("Log 2".to_string());
        assert_eq!(app.current_log_index, 1);
        assert_eq!(app.log_scroll_offset, 0);
    }

    #[test]
    fn scroll_operations() {
        let mut app = PlanApp::new();
        app.push_log("Line 1\nLine 2\nLine 3\nLine 4".to_string());

        app.scroll_down(2);
        assert_eq!(app.log_scroll_offset, 2);

        app.scroll_up(1);
        assert_eq!(app.log_scroll_offset, 1);

        app.scroll_up(10); // Saturates at 0
        assert_eq!(app.log_scroll_offset, 0);
    }

    #[test]
    fn reset_submit() {
        let mut app = PlanApp::new();
        app.should_submit = true;
        app.reset_submit();
        assert!(!app.should_submit);
    }

    #[test]
    fn set_processing_enables_state() {
        let mut app = PlanApp::new();
        assert!(!app.processing);

        // Set up questions and answers before processing
        app.set_questions(vec![
            create_test_question("q1", true),
            create_test_question("q2", true),
        ]);
        app.answers.push(Answer {
            question_id: "q1".to_string(),
            value: "A".to_string(),
        });
        app.answers.push(Answer {
            question_id: "q2".to_string(),
            value: "B".to_string(),
        });

        app.set_processing(true, "Testing...");
        assert!(app.processing);
        assert_eq!(app.processing_message, "Testing...");
        assert_eq!(app.spinner_frame, 0);
        // Verify counts were captured
        assert_eq!(app.submitted_count, 2);
        assert_eq!(app.submitted_total, 2);
    }

    #[test]
    fn set_processing_clears_state() {
        let mut app = PlanApp::new();
        app.set_processing(true, "Working...");
        app.spinner_frame = 5;

        app.set_processing(false, "");
        assert!(!app.processing);
        assert_eq!(app.processing_message, "");
    }

    #[test]
    fn advance_spinner_cycles() {
        let mut app = PlanApp::new();
        assert_eq!(app.spinner_frame, 0);

        app.advance_spinner();
        assert_eq!(app.spinner_frame, 1);

        // Cycle through all frames
        for _ in 0..7 {
            app.advance_spinner();
        }
        assert_eq!(app.spinner_frame, 0); // Should wrap around
    }

    #[test]
    fn spinner_char_returns_braille() {
        let mut app = PlanApp::new();
        // First frame should be '⠋'
        assert_eq!(app.spinner_char(), '⠋');

        app.spinner_frame = 4;
        assert_eq!(app.spinner_char(), '⠼');
    }
}
