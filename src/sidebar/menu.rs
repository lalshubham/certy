#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuItem {
    NewFile,
    NewFolder,
    OpenFile,
    OpenFolder,
    Save,
    CloseFolder,
    Exit,
}

pub const MENU_ITEMS: [(MenuItem, &str); 7] = [
    (MenuItem::NewFile, "New File"),
    (MenuItem::NewFolder, "New Folder"),
    (MenuItem::OpenFile, "Open File"),
    (MenuItem::OpenFolder, "Open Folder"),
    (MenuItem::Save, "Save"),
    (MenuItem::CloseFolder, "Close Folder"),
    (MenuItem::Exit, "Exit"),
];
