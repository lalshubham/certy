#[derive(Clone, Debug)]
pub enum EditAction {
    Insert { char_idx: usize, text: String },
    Delete { char_idx: usize, text: String },
}

#[derive(Default)]
pub struct History {
    undo_stack: Vec<EditAction>,
    redo_stack: Vec<EditAction>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn record(&mut self, action: EditAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    pub fn pop_undo(&mut self) -> Option<EditAction> {
        self.undo_stack.pop()
    }

    pub fn push_redo(&mut self, action: EditAction) {
        self.redo_stack.push(action);
    }

    pub fn pop_redo(&mut self) -> Option<EditAction> {
        self.redo_stack.pop()
    }

    pub fn push_undo(&mut self, action: EditAction) {
        self.undo_stack.push(action);
    }
}
