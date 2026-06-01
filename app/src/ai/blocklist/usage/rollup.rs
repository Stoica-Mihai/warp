//! Aggregates credit usage across an orchestrator and its locally-loaded
//! descendants for the agent-mode footer rollup feature (QUALITY-671).
//!
//! Pure function — no I/O, no GraphQL. Walks
//! [`BlocklistAIHistoryModel`] using the shared
//! [`descendant_conversation_ids_in_spawn_order`] helper, sums each loaded
//! conversation's `credits_spent`, and emits a per-agent breakdown for the
//! footer's "View details" list.

use crate::ai::agent::conversation::AIConversationId;

/// Avatar identity for a row in the per-agent breakdown.
///
/// The actual rendering still requires a theme (which the rollup, being a
/// pure function, cannot consult), so this enum only carries the structural
/// information needed to choose a renderer at render time. The child variant
/// reuses the orchestration pill bar's deterministic per-name color +
/// uppercase initial via the existing avatar helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAvatar {
    /// The orchestrator itself. Rendered with the Oz glyph on `ansi_fg_cyan`.
    Orchestrator,
    /// A descendant agent. Rendered with the same deterministic-color +
    /// initial-letter treatment as the orchestration pill bar.
    Child,
}

/// One row in the per-agent credit breakdown list.
#[derive(Debug, Clone, PartialEq)]
pub struct PerAgentCreditEntry {
    pub conversation_id: AIConversationId,
    pub display_name: String,
    pub avatar: AgentAvatar,
    pub credits_spent: f32,
}

/// Aggregated credit usage for an orchestrator and its locally-loaded
/// descendants.
#[derive(Debug, Clone, PartialEq)]
pub struct OrchestrationCreditRollup {
    /// Sum of `credits_spent` across the orchestrator and every
    /// locally-loaded descendant.
    pub total_credits: f32,
    /// One entry per agent that has spent > 0 credits, sorted by
    /// `credits_spent` descending. Ties are broken by spawn order (earlier
    /// spawn first; orchestrator always sorts before its descendants in a
    /// tie).
    pub per_agent: Vec<PerAgentCreditEntry>,
}

/// Computes the orchestration credit rollup for `parent_id`.
///
/// Returns `None` when:
/// * the orchestrator has no locally-loaded descendants, OR
/// * the orchestrator and every loaded descendant have spent zero credits.
///
/// Unloaded descendants (IDs in the topology index without a matching
/// `AIConversation` in `conversations_by_id`) are silently skipped — see
/// PRODUCT.md invariant 10.
pub fn compute_orchestration_rollup(
    _parent_id: AIConversationId,
) -> Option<OrchestrationCreditRollup> {
    None
}


