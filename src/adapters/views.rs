use serde::Serialize;

/// View model for displaying detailed yak information (used in `yx show`)
#[derive(Debug, Clone, Serialize)]
pub struct YakDetailView {
    /// Immutable yak identifier (e.g. "my-yak-a1b2")
    pub id: String,
    /// Ancestor yaks with id, name, and state (root-first)
    pub breadcrumb: Vec<YakChildView>,
    pub name: String,
    pub state: String,
    /// Formatted date string
    pub created_at: String,
    /// Author name
    pub created_by: String,
    /// Sorted by done-state then name
    pub children: Vec<YakChildView>,
    /// Single-line custom fields (title-cased key, value)
    pub short_fields: Vec<(String, String)>,
    /// Multi-line custom fields (title-cased key, value)
    pub long_fields: Vec<(String, String)>,
    /// Formatted tag strings
    pub tags: Vec<String>,
    /// Yak context text
    pub context: Option<String>,
    /// Whether context exists and is non-empty
    pub has_context: bool,
}

/// View model for a child yak in the detail view (also used for breadcrumb ancestors)
#[derive(Debug, Clone, Serialize)]
pub struct YakChildView {
    pub id: String,
    pub name: String,
    pub state: String,
}

/// View model for explicit blocker information in JSON list output.
#[derive(Debug, Clone, Serialize)]
pub struct YakBlockerView {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// View model for displaying the yak tree structure (used in `yx ls`)
#[derive(Debug, Clone, Serialize)]
pub struct YakTreeView {
    pub nodes: Vec<YakTreeNode>,
    /// "pretty", "markdown", "plain", "ids"
    pub format: String,
    pub is_empty: bool,
}

/// View model for a node in the yak tree
#[derive(Debug, Clone, Serialize)]
pub struct YakTreeNode {
    pub name: String,
    pub full_path: String,
    pub id: String,
    pub state: String,
    /// Derived actionable status: true when the yak is todo and all direct children are done.
    pub ready: bool,
    /// Presentation hint: true when any descendant is actively wip.
    #[serde(skip)]
    pub has_wip_descendant: bool,
    pub blocked_by: Vec<YakBlockerView>,
    pub context: Option<String>,
    pub parent_id: Option<String>,
    pub fields: std::collections::HashMap<String, String>,
    /// Formatted tag strings (with @ prefix for display, without for JSON)
    pub tags: Vec<String>,
    pub depth: usize,
    /// Tree drawing connector like "├─ " or "╰─ " (for pretty format)
    pub connector: String,
    /// Ancestor continuation lines (for pretty format)
    pub prefix: String,
    /// Nested children
    pub children: Vec<YakTreeNode>,
}

/// View model for a log entry
#[derive(Debug, Clone, Serialize)]
pub struct LogEntryView {
    /// Narrative spans
    pub narrative: Vec<NarrativeSpanView>,
    /// Pre-formatted relative timestamp
    pub relative_time: String,
    pub event_id: String,
    pub commit_sha: Option<String>,
}

/// View model for a narrative span
#[derive(Debug, Clone, Serialize)]
pub struct NarrativeSpanView {
    pub text: String,
    pub bold: bool,
}

/// User-facing messages with different severity levels
#[derive(Debug, Clone, Serialize)]
pub enum Message {
    Hint(String),
    Success(String),
    Info(String),
    Warn(String),
}
