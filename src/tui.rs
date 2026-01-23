use ratatui::DefaultTerminal;

pub fn init_terminal() -> DefaultTerminal {
    ratatui::init()
}
pub fn restore_terminal() {
    ratatui::restore()
}
