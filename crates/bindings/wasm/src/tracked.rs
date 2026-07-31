//! A [`Document`] behind a monotonic edit counter.
//!
//! A binding handle is a *reference*, not a value: a consumer holds one across a
//! session and asks "did this change since I last looked?". The core `Document`
//! cannot answer — it has value semantics (`Clone`, `PartialEq`, a serde
//! round-trip), and a counter on a value type is the history-dependent ambient
//! state [DOCUMENT_STORAGE.md] rules out for `$id`, for the same reason: two
//! equal documents reached different ways would carry different counters. So the
//! counter lives on the handle, where history is the point.
//!
//! [`revision`](Tracked::revision) is bumped by [`edit`](Tracked::edit), the one
//! door to `&mut Document`. `Deref` serves reads; `DerefMut` is deliberately
//! absent, so a mutator that forgets to go through `edit` does not compile
//! rather than silently under-counting. The bump is therefore *pessimistic* — a
//! mutator that takes the door and then fails its validation still counts — which
//! is the safe direction: an over-count costs a consumer one redundant recompute,
//! an under-count costs it a missed one.

use std::ops::Deref;

use quillmark_core::Document;

/// A [`Document`] paired with the count of edits taken against this handle.
#[derive(Debug)]
pub struct Tracked {
    doc: Document,
    revision: u64,
}

impl Tracked {
    /// Wrap `doc` at revision 0.
    pub fn new(doc: Document) -> Self {
        Self { doc, revision: 0 }
    }

    /// Mutable access, counting the edit — the only way to reach `&mut Document`.
    pub fn edit(&mut self) -> &mut Document {
        self.revision += 1;
        &mut self.doc
    }

    /// Swap the whole document, counting the edit — the `loadJson` door, where a
    /// mutator would be replacing rather than editing.
    pub fn replace(&mut self, doc: Document) {
        self.revision += 1;
        self.doc = doc;
    }

    /// Edits taken against this handle. Monotonic, and exact as an `f64` for any
    /// reachable count.
    pub fn revision(&self) -> f64 {
        self.revision as f64
    }
}

impl Deref for Tracked {
    type Target = Document;

    fn deref(&self) -> &Document {
        &self.doc
    }
}
