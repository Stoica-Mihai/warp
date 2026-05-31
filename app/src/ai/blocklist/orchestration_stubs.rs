// Stubs for deleted orchestration cluster (orchestration_events, orchestration_event_streamer,
// orchestration_topology, orchestration_conversation_links, task_status_sync_model,
// local_shared_session_link_model).

use warpui::{Entity, ModelContext, SingletonEntity};

use warpui::AppContext;

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::LifecycleEventType;

pub enum SendEventResult {
    LifecycleSent,
    LifecycleDropped,
    Error(String),
}

pub struct OrchestrationEventService;

impl OrchestrationEventService {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self }

    pub fn register_lifecycle_subscription(
        &mut self,
        _child_conversation_id: AIConversationId,
        _parent_agent_id: String,
        _lifecycle_subscription: Option<Vec<LifecycleEventType>>,
    ) {}

    pub fn emit_child_killed(
        &mut self,
        _conversation_id: AIConversationId,
        _ctx: &mut ModelContext<Self>,
    ) -> SendEventResult {
        SendEventResult::LifecycleSent
    }
}

impl Entity for OrchestrationEventService { type Event = (); }
impl SingletonEntity for OrchestrationEventService {}

pub struct OrchestrationEventStreamer;

impl OrchestrationEventStreamer {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self }

    pub fn mark_conversation_killed(
        &mut self,
        _conversation_id: AIConversationId,
        _ctx: &mut ModelContext<Self>,
    ) {}
}

impl Entity for OrchestrationEventStreamer { type Event = (); }
impl SingletonEntity for OrchestrationEventStreamer {}

pub struct TaskStatusSyncModel;

impl TaskStatusSyncModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self }
}

impl Entity for TaskStatusSyncModel { type Event = (); }
impl SingletonEntity for TaskStatusSyncModel {}

pub fn descendant_conversation_ids_in_spawn_order(
    _history: &super::history_model::BlocklistAIHistoryModel,
    _parent_id: AIConversationId,
) -> Vec<AIConversationId> {
    vec![]
}

pub fn collect_descendant_conversation_ids_in_spawn_order(
    _history: &super::history_model::BlocklistAIHistoryModel,
    _parent_id: AIConversationId,
    _descendants: &mut Vec<AIConversationId>,
) {}

pub fn conversation_navigation_card_with_icon(
    _icon: impl std::fmt::Debug,
    _title: String,
    _subtitle: Option<String>,
    _on_click: impl std::any::Any + 'static,
    _mouse_state: impl std::any::Any,
    _expands_to_max_width: bool,
    _extra_trailing: Option<Box<dyn warpui::Element>>,
    _app: &AppContext,
) -> Box<dyn warpui::Element> {
    warpui::elements::Empty::new().finish()
}

pub fn conversation_id_for_agent_id(
    _agent_id: &str,
    _app: &AppContext,
) -> Option<AIConversationId> {
    None
}

pub fn dispatch_focus_or_open_child_agent_pane(
    _conversation_id: AIConversationId,
    _ctx: &mut warpui::ViewContext<crate::pane_group::PaneGroup>,
) {}

pub struct LocalSharedSessionLinkModel;

impl LocalSharedSessionLinkModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self { Self }
}

impl Entity for LocalSharedSessionLinkModel { type Event = (); }
impl SingletonEntity for LocalSharedSessionLinkModel {}
