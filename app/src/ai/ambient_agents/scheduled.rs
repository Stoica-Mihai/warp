use serde::{Deserialize, Serialize};

use super::AgentConfigSnapshot;
use crate::cloud_object::model::generic_string_model::{
    GenericStringModel, GenericStringObjectId, StringModel,
};
use crate::cloud_object::model::json_model::{JsonModel, JsonSerializer};
use crate::cloud_object::{
    GenericCloudObject, GenericStringObjectFormat,
    GenericStringObjectUniqueKey, JsonObjectType,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
/// A ScheduledAmbientAgent represents configuration for ambient agents that run on a cron schedule.
pub struct ScheduledAmbientAgent {
    /// Agent name
    #[serde(default)]
    pub name: String,
    /// Cron schedule expression
    #[serde(default)]
    pub cron_schedule: String,
    /// Whether the scheduled agent is enabled
    #[serde(default)]
    pub enabled: bool,
    /// The prompt to use for the scheduled agent
    #[serde(default)]
    pub prompt: String,
    /// The latest failure to execute this scheduled agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_spawn_error: Option<String>,
    /// Configuration for how the ambient agent should run.
    #[serde(default, skip_serializing_if = "AgentConfigSnapshot::is_empty")]
    pub agent_config: AgentConfigSnapshot,
}

pub type CloudScheduledAmbientAgent =
    GenericCloudObject<GenericStringObjectId, CloudScheduledAmbientAgentModel>;
pub type CloudScheduledAmbientAgentModel =
    GenericStringModel<ScheduledAmbientAgent, JsonSerializer>;

impl StringModel for ScheduledAmbientAgent {
    type CloudObjectType = CloudScheduledAmbientAgent;

    fn model_type_name(&self) -> &'static str {
        "Scheduled ambient agent"
    }

    fn should_enforce_revisions() -> bool {
        true
    }

    fn model_format() -> GenericStringObjectFormat {
        GenericStringObjectFormat::Json(JsonObjectType::ScheduledAmbientAgent)
    }

    fn display_name(&self) -> String {
        self.name.clone()
    }


    fn uniqueness_key(&self) -> Option<GenericStringObjectUniqueKey> {
        None
    }

    fn should_show_activity_toasts() -> bool {
        false
    }

    fn warn_if_unsaved_at_quit() -> bool {
        true
    }
}

impl JsonModel for ScheduledAmbientAgent {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::ScheduledAmbientAgent
    }
}




