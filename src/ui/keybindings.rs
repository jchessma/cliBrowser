use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    ScrollDown(usize),
    ScrollUp(usize),
    PageDown,
    PageUp,
    GoToTop,
    GoToBottom,
    NextLink,
    PrevLink,
    Follow,
    Back,
    Forward,
    OpenUrl,
    Reload,
    Bookmark,
    ShowBookmarks,
    ShowHistory,
    ShowHelp,
    CopyLinkUrl,
    ToggleCheck,
    None,
}

pub fn map_key(key: KeyEvent) -> Action {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('q'))
        | (KeyModifiers::CONTROL, KeyCode::Char('c')) => Action::Quit,

        // Scrolling
        (KeyModifiers::NONE, KeyCode::Char('j'))
        | (KeyModifiers::NONE, KeyCode::Down) => Action::ScrollDown(1),
        (KeyModifiers::NONE, KeyCode::Char('k'))
        | (KeyModifiers::NONE, KeyCode::Up) => Action::ScrollUp(1),

        (KeyModifiers::NONE, KeyCode::Char('d')) => Action::ScrollDown(10),
        (KeyModifiers::NONE, KeyCode::Char('u')) => Action::ScrollUp(10),

        (KeyModifiers::NONE, KeyCode::PageDown) => Action::PageDown,
        (KeyModifiers::NONE, KeyCode::PageUp) => Action::PageUp,
        (KeyModifiers::NONE, KeyCode::Char(' ')) => Action::ToggleCheck,

        (KeyModifiers::NONE, KeyCode::Char('g')) => Action::GoToTop,
        (KeyModifiers::NONE, KeyCode::Char('G')) => Action::GoToBottom,

        // Link navigation (Tab / Shift+Tab)
        (KeyModifiers::NONE, KeyCode::Tab) => Action::NextLink,
        (KeyModifiers::SHIFT, KeyCode::BackTab) => Action::PrevLink,

        // Follow link / open
        (KeyModifiers::NONE, KeyCode::Enter) => Action::Follow,

        // History navigation
        (KeyModifiers::NONE, KeyCode::Char('H'))
        | (KeyModifiers::ALT, KeyCode::Left)
        | (KeyModifiers::NONE, KeyCode::Backspace) => Action::Back,
        (KeyModifiers::NONE, KeyCode::Char('L'))
        | (KeyModifiers::ALT, KeyCode::Right) => Action::Forward,

        // Navigation
        (KeyModifiers::NONE, KeyCode::Char('o')) => Action::OpenUrl,

        (KeyModifiers::NONE, KeyCode::Char('r'))
        | (KeyModifiers::NONE, KeyCode::F(5)) => Action::Reload,

        // Bookmarks / UI
        (KeyModifiers::NONE, KeyCode::Char('b')) => Action::Bookmark,
        (KeyModifiers::NONE, KeyCode::Char('B')) => Action::ShowBookmarks,
        (KeyModifiers::NONE, KeyCode::Char('h')) => Action::ShowHistory,
        (KeyModifiers::NONE, KeyCode::Char('?')) => Action::ShowHelp,

        (KeyModifiers::NONE, KeyCode::Char('y')) => Action::CopyLinkUrl,

        _ => Action::None,
    }
}
