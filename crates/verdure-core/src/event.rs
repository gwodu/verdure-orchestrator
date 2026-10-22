//! Progress and lifecycle events, and the callback that receives them. Every
//! long operation (download, model load, extraction) reports through this and
//! can be cancelled through it — spec rule (d), knowledge/26.

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Bytes or steps done out of total (total 0 = unknown).
    Progress { done: u64, total: u64 },
    /// A bundle below the device's tier was chosen, and why.
    Downgraded {
        from: String,
        to: String,
        reason: String,
    },
    /// Thermal pressure: the thread count was halved.
    Throttled { threads: u32 },
    /// Memory pressure: the LLM was unloaded (ASR keeps running).
    ModelUnloaded { reason: String },
    /// A plain, user-showable status line.
    Status(String),
}

/// The host's ear. Implemented by the vendor's code (through the bindings) or
/// by `NoProgress` when nobody is listening.
pub trait Progress: Send + Sync {
    fn on_event(&self, event: Event);
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Listens to nothing and never cancels.
pub struct NoProgress;

impl Progress for NoProgress {
    fn on_event(&self, _event: Event) {}
}

/// A test/CLI listener that records what it heard.
#[derive(Default)]
pub struct Recorder {
    pub events: std::sync::Mutex<Vec<Event>>,
    pub cancel_after: Option<usize>,
}

impl Progress for Recorder {
    fn on_event(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
    fn is_cancelled(&self) -> bool {
        self.cancel_after
            .map(|n| self.events.lock().unwrap().len() >= n)
            .unwrap_or(false)
    }
}
