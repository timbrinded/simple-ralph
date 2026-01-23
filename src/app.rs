use ratatui::{
    Frame,
    widgets::{Block, Borders, Paragraph},
};

pub struct App {
    pub prd_name: String,
    pub remaining_tasks: usize,
    pub completed_tasks: usize,
    pub loop_count: u64,
    pub should_quit: bool,
    pub status_message: String,
    pub claude_output: String,
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
            claude_output: String::new(),
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        let content = format!(
            "PRD: {}\nProgress: {}/{} tasks complete\nLoop: #{}\n\nStatus: {}\n\n{}",
            self.prd_name,
            self.completed_tasks,
            self.completed_tasks + self.remaining_tasks,
            self.loop_count,
            self.status_message,
            if self.claude_output.is_empty() { "" } else { "─── Output ───\n" },
        );

        let paragraph = Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title(" simple ralph "));

        frame.render_widget(paragraph, area);
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
}
