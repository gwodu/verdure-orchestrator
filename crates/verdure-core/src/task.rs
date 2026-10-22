//! What the vendor's app does with speech. A bundle is built for one task;
//! `recommend(task)` picks it per device tier (knowledge/26, Part 4).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    /// Clinical dictation and ambient notes: ASR + cleanup + note/facts extraction.
    Scribe,
    /// Tutoring over device-resident content: chat + RAG over the vendor's corpus.
    Tutor,
    /// Community-health-worker visit summaries → prefilled form fields.
    ChwForms,
}

impl Task {
    pub fn all() -> [Task; 3] {
        [Task::Scribe, Task::Tutor, Task::ChwForms]
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Task::Scribe => "scribe",
            Task::Tutor => "tutor",
            Task::ChwForms => "chw_forms",
        })
    }
}
