//! Rule-application log, populated only with the `trace` feature.

/// Ordered record of rule applications for one key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// Steps in the order they ran.
    pub steps: Vec<TraceStep>,
}

/// One rule application or a recursion stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceStep {
    /// A rule applied; `rank` is its one-based position in priority order.
    Applied {
        /// Index of the rule in the keyboard.
        rule: usize,
        /// One-based position in priority order.
        rank: usize,
        /// Whether the application ran during recursion.
        recursive: bool,
        /// Text the right-hand side produced.
        output: String,
        /// Text each left-hand-side capture matched, in capture order.
        captures: Vec<String>,
    },
    /// Recursion stopped at the iteration cap.
    Capped,
}

/// Collects steps only when the `trace` feature is enabled.
#[derive(Debug, Default)]
pub struct Recorder {
    steps: Vec<TraceStep>,
}

impl Recorder {
    /// Stores the step produced by `step` when tracing is enabled.
    pub fn record(&mut self, step: impl FnOnce() -> TraceStep) {
        if cfg!(feature = "trace") {
            self.steps.push(step());
        }
    }

    /// Returns the collected trace, or `None` without the feature.
    pub fn finish(self) -> Option<Trace> {
        cfg!(feature = "trace").then_some(Trace { steps: self.steps })
    }
}
