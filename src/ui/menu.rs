/// Main menu — corresponds to WL_MENU.C.
///
/// Menu structure: each item has a label and an action.
/// Navigation uses up/down arrows; Enter selects.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    LoadGame,
    SaveGame,
    ViewScores,
    BackToDemo,
    QuitGame,
}

pub struct MenuItem {
    pub label: &'static str,
    pub action: MenuAction,
    pub enabled: bool,
}

pub struct Menu {
    pub items: Vec<MenuItem>,
    pub cursor: usize,
}

impl Menu {
    pub fn main_menu() -> Self {
        Self {
            cursor: 0,
            items: vec![
                MenuItem { label: "New Game",    action: MenuAction::NewGame,    enabled: true },
                MenuItem { label: "Load Game",   action: MenuAction::LoadGame,   enabled: true },
                MenuItem { label: "Save Game",   action: MenuAction::SaveGame,   enabled: false },
                MenuItem { label: "View Scores", action: MenuAction::ViewScores, enabled: true },
                MenuItem { label: "Back to Demo",action: MenuAction::BackToDemo, enabled: true },
                MenuItem { label: "Quit Game",   action: MenuAction::QuitGame,   enabled: true },
            ],
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        } else {
            self.cursor = self.items.len() - 1;
        }
    }

    pub fn cursor_down(&mut self) {
        self.cursor = (self.cursor + 1) % self.items.len();
    }

    pub fn select(&self) -> Option<MenuAction> {
        let item = self.items.get(self.cursor)?;
        if item.enabled { Some(item.action) } else { None }
    }

    /// Draw the menu into an RGBA framebuffer.
    /// TODO: use actual game font and graphics once asset loading is in place.
    pub fn draw(&self, _fb: &mut [u8], _width: usize, _height: usize) {
        // Placeholder — draw a filled rectangle for each item,
        // highlighted for the current cursor position.
    }
}
