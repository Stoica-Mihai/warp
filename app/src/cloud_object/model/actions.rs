use std::collections::HashMap;

use chrono::{DateTime, Utc};
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::persistence::model::PersistedObjectAction;
use crate::server::ids::{parse_sqlite_id_to_uid, HashedSqliteId, ObjectUid};

pub enum ObjectActionsEvent {}

/// The type of action that occurred on an object, such as an execution, selection, so on
/// and so forth.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectActionType {
    Execute,
}

// In order to convert from a graphql type and from a SQLite read, the action type
// implements to_string().
//
// Temporarily suppress clippy warnings about the `ToString` impl until we
// move `ObjectType` away from using `std::fmt::Display` for serialization.
#[allow(clippy::to_string_trait_impl)]
impl ToString for ObjectActionType {
    fn to_string(&self) -> String {
        match self {
            ObjectActionType::Execute => String::from("EXECUTE"),
        }
    }
}

/// We track object actions, both those that have been sent to the server and not, through this
/// type. A single ObjectAction represents an object_id, action pair and a subtype that contains data
/// about the action(s). Each ObjectAction either represents one action or a summary of identical actions
/// that occurred at different times. We summarize old actions in order to save memory footprint on the client.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectAction {
    pub action_type: ObjectActionType,
    pub uid: ObjectUid,
    pub hashed_sqlite_id: HashedSqliteId,
    // This action either represents one action or a consolidation of multiple actions.
    pub action_subtype: ObjectActionSubtype,
}

impl ObjectAction {
    pub fn is_pending(&self) -> bool {
        match self.action_subtype {
            ObjectActionSubtype::SingleAction { pending, .. } => pending,
            _ => false,
        }
    }
}

impl TryFrom<PersistedObjectAction> for ObjectAction {
    type Error = ();

    fn try_from(other: PersistedObjectAction) -> Result<Self, Self::Error> {
        // Each persisted object action is either a single action or a bundled action.
        // If there's any inconsistencies from the SQL row, we return an error.
        let action_subtype = if let Some(count) = other.count {
            let oldest_timestamp = other
                .oldest_timestamp
                .as_ref()
                .map(|time| time.and_utc())
                .ok_or(())?;
            let latest_timestamp = other
                .latest_timestamp
                .as_ref()
                .map(|time| time.and_utc())
                .ok_or(())?;

            // When the db row is a bundled action, the processed_at_timestamp field refers
            // to the latest processed_at_timestamp in the bundle. Because bundled actions come
            // from the server, this is a value, not an option.
            let latest_processed_at_timestamp = other
                .processed_at_timestamp
                .as_ref()
                .map(|time| time.and_utc())
                .ok_or(())?;
            ObjectActionSubtype::BundledActions {
                count,
                oldest_timestamp,
                latest_timestamp,
                latest_processed_at_timestamp,
            }
        } else {
            let timestamp = other
                .timestamp
                .as_ref()
                .map(|time| time.and_utc())
                .ok_or(())?;
            let pending = other.pending.ok_or(())?;

            // The processed_at_timestamp is still None when the action hasn't been synced.
            let processed_at_timestamp = other
                .processed_at_timestamp
                .as_ref()
                .map(|time| time.and_utc());
            ObjectActionSubtype::SingleAction {
                timestamp,
                data: other.data,
                pending,
                processed_at_timestamp,
            }
        };

        // The object_sync_id stored in SQLite is the hashed id that's used to index into the ObjectActions
        // model.
        let hashed_object_id = other.hashed_object_id;
        let action_type = match other.action.as_str() {
            s if s == ObjectActionType::Execute.to_string() => ObjectActionType::Execute,
            _ => return Err(()),
        };

        // NOTE: This is needed since we only store the sqlite hash, but we need the uid (the second part of the hash)
        // to index into CloudModel and store the object actions in memory.
        let uid = parse_sqlite_id_to_uid(hashed_object_id.clone())?;

        Ok(ObjectAction {
            uid: uid.to_string(),
            hashed_sqlite_id: hashed_object_id,
            action_type,
            action_subtype,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjectActionSubtype {
    SingleAction {
        // When the action occurred.
        timestamp: DateTime<Utc>,

        // When the action was processed by the server (used to order actions against eachother).
        // None if the action has not been synced.
        processed_at_timestamp: Option<DateTime<Utc>>,

        // A JSON representation of anything else we might want to track about the action.
        // For example, the exit code of a workflow execution.
        data: Option<String>,

        // Whether or not this action has been successfully synced to the server.
        pending: bool,
    },
    BundledActions {
        // The number of distinct actions that are coalesced into one entry here.
        count: i32,

        // The timestamp of the oldest action within this bundle.
        oldest_timestamp: DateTime<Utc>,

        // The timestamp of the most recent action within the bundle.
        latest_timestamp: DateTime<Utc>,

        // The most recent processed_at timestamp contained in the bundle (used to order actions and determine
        // how up-to-date the client's actions are.)
        latest_processed_at_timestamp: DateTime<Utc>,
    },
}

/// A singleton model representing the actions that have occurred on a per-object basis. These
/// represent actions taken by the user or by teammates. The actions have a pending status that is
/// true when the server doesn't know about it and is false anytime after the action is successfully
/// synced.
pub struct ObjectActions {
    #[allow(dead_code)]
    object_actions_by_id: HashMap<ObjectUid, Vec<ObjectAction>>,
}

impl ObjectActions {
    /// Accepts a vector of object actions read out of SQLite.
    pub fn new(persisted_actions: Vec<ObjectAction>) -> Self {
        // Partitions the actions by object id and plops them into the map.
        let object_actions_by_id = persisted_actions.into_iter().fold(
            HashMap::new(),
            |mut map: HashMap<ObjectUid, Vec<ObjectAction>>, object_action| {
                map.entry(object_action.uid.clone())
                    .or_default()
                    .push(object_action);
                map
            },
        );

        Self {
            object_actions_by_id,
        }
    }

    /// Insert a single action into the model. Returns the created action.
    pub fn insert_action(
        &mut self,
        uid: ObjectUid,
        hashed_sqlite_id: HashedSqliteId,
        action_type: ObjectActionType,
        data: Option<String>,
        timestamp: DateTime<Utc>,
        ctx: &mut ModelContext<Self>,
    ) -> ObjectAction {
        // Create an action with pending=true.
        let action = ObjectAction {
            action_type,
            uid: uid.clone(),
            hashed_sqlite_id,
            action_subtype: ObjectActionSubtype::SingleAction {
                timestamp,
                data,
                pending: true,
                processed_at_timestamp: None,
            },
        };

        // Insert the action into the model.
        self.object_actions_by_id
            .entry(uid)
            .or_default()
            .push(action.clone());

        ctx.notify();

        action
    }

    pub fn delete_actions_for_object(&mut self, uid: &ObjectUid, ctx: &mut ModelContext<Self>) {
        self.object_actions_by_id.remove(uid);
        ctx.notify()
    }

}

impl Entity for ObjectActions {
    type Event = ObjectActionsEvent;
}

impl SingletonEntity for ObjectActions {}
