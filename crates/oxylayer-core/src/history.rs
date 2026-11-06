use crate::document::Document;

/// Non-destructive history / undo-redo manager
#[derive(Debug, Clone)]
pub struct HistoryManager {
    undo_stack: Vec<Document>,
    redo_stack: Vec<Document>,
    max_depth: usize,
}

impl HistoryManager {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::with_capacity(max_depth),
            redo_stack: Vec::new(),
            max_depth: max_depth.max(1),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Records current document state onto the undo stack prior to a mutation
    pub fn push_snapshot(&mut self, current_doc: &Document) {
        if self.undo_stack.len() >= self.max_depth {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(current_doc.clone());
        self.redo_stack.clear();
    }

    /// Performs undo, swapping current doc with previous state
    pub fn undo(&mut self, current_doc: &mut Document) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let next = std::mem::replace(current_doc, prev);
            self.redo_stack.push(next);
            true
        } else {
            false
        }
    }

    /// Performs redo, restoring next state
    pub fn redo(&mut self, current_doc: &mut Document) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let prev = std::mem::replace(current_doc, next);
            self.undo_stack.push(prev);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
