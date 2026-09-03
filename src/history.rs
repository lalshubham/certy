use std::collections::VecDeque;

#[derive(Clone)]
pub enum EditAction {
    Insert { char_idx: usize, text: String },
    Delete { char_idx: usize, text: String },
}

pub struct History {
    undo_stack: VecDeque<EditAction>,
    redo_stack: Vec<EditAction>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(64),
            redo_stack: Vec::new(),
        }
    }

    pub fn record(&mut self, action: EditAction) {
        if self.undo_stack.len() >= 1000 {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(action);
        self.redo_stack.clear();
    }

    pub fn pop_undo(&mut self) -> Option<EditAction> {
        self.undo_stack.pop_back()
    }

    pub fn push_undo(&mut self, action: EditAction) {
        self.undo_stack.push_back(action);
    }

    pub fn pop_redo(&mut self) -> Option<EditAction> {
        self.redo_stack.pop()
    }

    pub fn push_redo(&mut self, action: EditAction) {
        self.redo_stack.push(action);
    }
}
