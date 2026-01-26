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

    /// Submit answer for current question
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
                self.answers.push(Answer {
                    question_id: q.id.clone(),
                    value,
                });
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
        !self.questions.is_empty() && self.answers.len() >= self.questions.len()
    }

    /// Get count of answered questions
    pub fn answered_count(&self) -> usize {
        self.answers.len()
    }

    /// Reset submit flag
    pub fn reset_submit(&mut self) {
        self.should_submit = false;
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

        // Build progress indicator for asking phase
        let progress_span = if self.phase == PlanPhase::Asking && !self.questions.is_empty() {
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

        let [question_area, options_area, input_area] = Layout::vertical([
            Constraint::Length(6),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(area);

        // Render current question
        if let Some(q) = self.questions.get(self.current_question) {
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

            // Render options if present
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
            }

            // Render freeform input if allowed
            if q.allow_freeform || q.options.is_none() {
                let input_style = if self.input_mode == InputMode::Editing {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let input_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .border_style(input_style)
                    .title(if self.input_mode == InputMode::Editing {
                        " Type your answer (Esc to finish) "
                    } else {
                        " Press 'i' to type custom answer "
                    })
                    .padding(Padding::horizontal(1));

                let input_widget = Paragraph::new(self.freeform_input.as_str())
                    .block(input_block)
                    .style(Style::default().fg(Color::White));

                frame.render_widget(input_widget, input_area);

                // Show cursor in editing mode
                if self.input_mode == InputMode::Editing {
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
