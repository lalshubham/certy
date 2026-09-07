use crate::sidebar::MenuItem;
use std::path::PathBuf;

pub enum ActionEvent {
    None,
    Redraw,
    ToggleSidebar,
    CloseAllFiles,
    SaveAllFiles,
    DiscardAllFiles,
    Menu(MenuItem),
    ToggleTerminal,
    OpenFile(PathBuf),
    SaveTab(usize),
    DiscardTab(usize),
    SaveAllAndExit,
    DiscardAllAndExit,
    CancelClose,
}

#[derive(Debug)]
pub enum AppEvent {
    SaveNewFile(PathBuf),
    OpenFile(PathBuf),
    OpenFolder(PathBuf),
    CreateFolder(PathBuf),
}
