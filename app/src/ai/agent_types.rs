//! Standalone agent data types that survive the deletion of the `ai/agent/` core.
//! Extracted from `ai/agent/mod.rs` + `ai/agent/task.rs` because non-AI features
//! (terminal blocks, code review, editor, persistence, CLI-agent sessions) still
//! consume them.

use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, Range};

use serde::{Deserialize, Serialize};
use warp_editor::render::model::LineCount;
use warp_multi_agent_api::diff_hunk as diff_hunk_api;

use crate::code_review::comments::{AttachedReviewComment as CodeReviewComment, ReviewCommentBatch};
use crate::secret_redaction::{find_secrets_in_text, SECRET_REDACTION_REPLACEMENT_CHARACTER};
use crate::terminal::shell::ShellType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(id: String) -> Self {
        TaskId(id)
    }
}

impl From<TaskId> for String {
    fn from(id: TaskId) -> Self {
        id.0
    }
}

impl Deref for TaskId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An ID for an AI action generated as part of an [`AIAgentOutput`].
///
/// The internal ID itself should be opaque to all callers. This ID may be relayed back to the AI with
/// the `AIAgentActionResult` from the action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AIAgentActionId(String);

impl From<String> for AIAgentActionId {
    fn from(value: String) -> Self {
        AIAgentActionId(value)
    }
}

impl From<AIAgentActionId> for String {
    fn from(value: AIAgentActionId) -> Self {
        value.0
    }
}

impl Display for AIAgentActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<crate::persistence::model::AIAgentActionId> for AIAgentActionId {
    fn from(value: crate::persistence::model::AIAgentActionId) -> Self {
        Self(value.0)
    }
}

impl From<AIAgentActionId> for crate::persistence::model::AIAgentActionId {
    fn from(value: AIAgentActionId) -> Self {
        crate::persistence::model::AIAgentActionId(value.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CancellationReason {
    /// The user explicitly cancelled without providing a follow-up.
    ManuallyCancelled,
    /// Warp automatically cancelled the local run so it could continue in Cloud Mode.
    AutomaticCloudHandoff,

    /// The user submitted a follow-up query during streaming which implicitly cancelled the current one.
    FollowUpSubmitted {
        is_for_same_conversation: bool,
    },

    /// The user executed a shell command in the middle of the response stream.
    UserCommandExecuted,

    /// The user reverted the conversation to a previous state, deleting exchanges.
    Reverted,

    // The user deleted the conversation while it was in progress.
    Deleted,

    /// The long-running command completed while the agent was still streaming.
    /// This should be treated as a successful completion, not a cancellation.
    OptimisticCLISubagentCompletion,
}

impl Display for CancellationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CancellationReason::ManuallyCancelled => write!(f, "manual cancellation"),
            CancellationReason::AutomaticCloudHandoff => write!(f, "automatic cloud handoff"),
            CancellationReason::FollowUpSubmitted { .. } => write!(f, "follow-up submission"),
            CancellationReason::UserCommandExecuted => write!(f, "user command execution"),
            CancellationReason::Reverted => write!(f, "revert"),
            CancellationReason::Deleted => write!(f, "deleted"),
            CancellationReason::OptimisticCLISubagentCompletion => {
                write!(f, "LRC command completed")
            }
        }
    }
}

impl CancellationReason {
    pub fn is_follow_up_for_same_conversation(&self) -> bool {
        matches!(
            self,
            CancellationReason::FollowUpSubmitted {
                is_for_same_conversation: true
            }
        )
    }

    pub fn is_manually_cancelled(&self) -> bool {
        matches!(self, CancellationReason::ManuallyCancelled)
    }

    pub fn is_reverted(&self) -> bool {
        matches!(self, CancellationReason::Reverted)
    }

    pub fn is_lrc_command_completed(&self) -> bool {
        matches!(self, CancellationReason::OptimisticCLISubagentCompletion)
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Shell(ShellType),
    Other(String),
}

impl ProgrammingLanguage {
    pub fn display_name(&self) -> String {
        match self {
            Self::Shell(shell_type) => shell_type.name().to_owned(),
            Self::Other(language) => language.to_lowercase(),
        }
    }

    /// Returns the file extension for the given programming language.
    // TODO(INT-605): Refactor so we don't have to edit this function and the `languages` crate.
    #[cfg_attr(target_family = "wasm", allow(unused))]
    pub fn to_extension(&self) -> Option<&str> {
        match self {
            // The arms below cover both canonical language names emitted by the agent (e.g.
            // "rust", "kotlin") and common markdown code-fence aliases (e.g. "rs", "kt") to keep
            // syntax highlighting working when the model uses either. The set of recognized
            // languages here is kept in sync with `SUPPORTED_LANGUAGES` in the `languages` crate.
            Self::Other(language) => match language.to_lowercase().as_str() {
                "rust" | "rs" => Some("rs"),
                "go" | "golang" => Some("go"),
                "python" | "py" => Some("py"),
                "javascript" | "js" => Some("js"),
                "typescript" | "ts" => Some("ts"),
                "jsx" => Some("jsx"),
                "tsx" => Some("tsx"),
                "yaml" | "yml" => Some("yaml"),
                "cpp" | "c++" => Some("cpp"),
                "java" => Some("java"),
                "groovy" => Some("java"),
                "shell" => Some("sh"),
                "c#" | "csharp" => Some("cs"),
                "html" => Some("html"),
                "css" => Some("css"),
                "c" => Some("c"),
                "json" => Some("json"),
                "jq" => Some("jq"),
                "hcl" | "terraform" | "tf" => Some("hcl"),
                "lua" => Some("lua"),
                "ruby" | "rb" => Some("rb"),
                "php" => Some("php"),
                "toml" => Some("toml"),
                "swift" => Some("swift"),
                "kotlin" | "kt" => Some("kt"),
                "powershell" => Some("ps1"),
                "elixir" => Some("exs"),
                "scala" => Some("scala"),
                "sql" => Some("sql"),
                "objective-c" | "objc" => Some("m"),
                "starlark" => Some("bzl"),
                "xml" => Some("xml"),
                "vue" => Some("vue"),
                "dockerfile" | "docker" | "containerfile" => Some("dockerfile"),
                _ => None,
            },
            Self::Shell(ShellType::PowerShell) => Some("ps1"),
            _ => None,
        }
    }

    /// Return whether this language is a shell language.
    /// This is used to determine whether to show the "execute in terminal" button.
    pub fn is_shell(&self) -> bool {
        matches!(self, Self::Shell(_))
    }
}

impl Display for ProgrammingLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgrammingLanguage::Shell(shell_type) => write!(f, "{}", shell_type.name()),
            ProgrammingLanguage::Other(language) => write!(f, "{}", language.to_lowercase()),
        }
    }
}

impl From<String> for ProgrammingLanguage {
    // Returns a programming language for a markdown language specifier
    fn from(value: String) -> Self {
        if let Some(shell_type) = ShellType::from_markdown_language_spec(value.as_str()) {
            ProgrammingLanguage::Shell(shell_type)
        } else {
            ProgrammingLanguage::Other(value)
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ImageContext {
    /// Base64-encoded image data.
    pub data: String,

    /// MIME type of the media content (e.g., "image/jpeg", "image/png")
    pub mime_type: String,

    pub file_name: String,

    /// Whether this image was exported from Figma, detected via
    /// the `Software: Figma` PNG metadata field.
    #[serde(default)]
    pub is_figma: bool,
}

impl std::fmt::Debug for ImageContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // We log dispatching typed actions (with `ImageContext` as an argument) and we don't want
        // to log any UGC in prod.
        f.debug_struct("ImageContext")
            .field("data", &"REDACTED_B64_IMAGE_DATA_UGC")
            .field("mime_type", &self.mime_type)
            .field("file_name", &"REDACTED_FILE_NAME_UGC")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrentHead {
    BranchName(String),
    HeadlessCommitSha(String),
}

impl CurrentHead {
    pub fn title(&self) -> String {
        match self {
            CurrentHead::BranchName(name) => name.clone(),
            CurrentHead::HeadlessCommitSha(sha) => {
                let short = sha.chars().take(7).collect::<String>();
                format!("Commit {short}")
            }
        }
    }
}

impl From<CurrentHead> for warp_multi_agent_api::CurrentRef {
    fn from(value: CurrentHead) -> Self {
        Self {
            r#ref: Some(match value {
                CurrentHead::BranchName(name) => {
                    warp_multi_agent_api::current_ref::Ref::BranchName(name)
                }
                CurrentHead::HeadlessCommitSha(sha) => {
                    warp_multi_agent_api::current_ref::Ref::HeadlessCommitSha(sha)
                }
            }),
        }
    }
}

impl From<CurrentHead> for diff_hunk_api::Current {
    fn from(value: CurrentHead) -> Self {
        match value {
            CurrentHead::BranchName(name) => diff_hunk_api::Current::CurrentBranchName(name),
            CurrentHead::HeadlessCommitSha(sha) => {
                diff_hunk_api::Current::CurrentHeadlessCommitSha(sha)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffBase {
    BranchName(String),
    HeadlessCommitSha(String),
    UncommittedChanges,
}

impl From<DiffBase> for warp_multi_agent_api::BaseRef {
    fn from(value: DiffBase) -> Self {
        Self {
            r#ref: Some(match value {
                DiffBase::BranchName(name) => warp_multi_agent_api::base_ref::Ref::BranchName(name),
                DiffBase::HeadlessCommitSha(sha) => {
                    warp_multi_agent_api::base_ref::Ref::HeadlessCommitSha(sha)
                }
                DiffBase::UncommittedChanges => {
                    warp_multi_agent_api::base_ref::Ref::UncommittedChanges(())
                }
            }),
        }
    }
}

impl From<DiffBase> for diff_hunk_api::Base {
    fn from(value: DiffBase) -> Self {
        match value {
            DiffBase::BranchName(branch_name) => diff_hunk_api::Base::BaseBranchName(branch_name),
            DiffBase::HeadlessCommitSha(sha) => diff_hunk_api::Base::BaseHeadlessCommitSha(sha),
            DiffBase::UncommittedChanges =>
            {
                #[warn(clippy::unit_arg)]
                diff_hunk_api::Base::UncommittedChanges(())
            }
        }
    }
}

/// A simplified diff hunk for use in DiffSet attachments
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSetHunk {
    pub line_range: Range<LineCount>,
    pub diff_content: String,
    pub lines_added: u32,
    pub lines_removed: u32,
}

impl DiffSetHunk {
    pub fn convert_to_api(self, file_path: String) -> warp_multi_agent_api::diff_set::DiffHunk {
        warp_multi_agent_api::diff_set::DiffHunk {
            file_path,
            line_range: Some(warp_multi_agent_api::FileContentLineRange {
                start: self.line_range.start.as_usize() as u32,
                end: self.line_range.end.as_usize() as u32,
            }),
            diff_content: self.diff_content,
            lines_added: self.lines_added,
            lines_removed: self.lines_removed,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentReviewCommentBatch {
    /// The review comments in this batch. Uses `code_review::comments::ReviewComment`
    /// because it contains full target information needed for API conversion and UI rendering.
    pub comments: Vec<CodeReviewComment>,
    /// All diff hunks that have comments in this batch attached to them, grouped by file name.
    pub diff_set: HashMap<String, Vec<DiffSetHunk>>,
}

impl AgentReviewCommentBatch {
    pub fn review_comments(&self) -> ReviewCommentBatch {
        ReviewCommentBatch::from_comments(self.comments.clone())
    }
}

/// Redact all detected secrets in-place within the given string.
pub(crate) fn redact_secrets(input: &mut String) {
    let mut secrets: Vec<_> = find_secrets_in_text(input)
        .into_iter()
        .map(|r| r.byte_range)
        .collect();
    // Replace from the end to preserve indices
    secrets.sort_by_key(|range| range.start);
    for range in secrets.into_iter().rev() {
        let replacement =
            SECRET_REDACTION_REPLACEMENT_CHARACTER.repeat(range.end.saturating_sub(range.start));
        input.replace_range(range.start..range.end, &replacement);
    }
}
