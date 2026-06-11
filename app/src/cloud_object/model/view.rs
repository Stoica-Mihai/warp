use std::cell::RefCell;
use std::collections::HashMap;

use warp_util::server_timestamp::ServerTimestamp;
use warpui::{Entity, ModelContext, SingletonEntity};

use super::persistence::{CloudModel, CloudModelEvent};
use crate::server::cloud_objects::update_manager::{
    OperationSuccessType, UpdateManager, UpdateManagerEvent,
};
use crate::server::ids::{ObjectUid, SyncId};

/// Singleton model for storing and querying the data and logic logic needed by various view, based on the information
/// stored in [CloudModel]. As a general, rule, any new API that requires logic beyond just retrieving the raw value
/// in [CloudModel], should be stored here. This includes logic such as object trashed status, the object current editor,
/// and object location.
///
/// Any API added to this model should be unit tested in model_test.rs
pub struct CloudViewModel {
    folder_timestamp_cache: FolderTimestampCache,
}

type FolderTimestampCache = RefCell<HashMap<SyncId, ServerTimestamp>>;

pub enum CloudViewModelEvent {
    /// A model change has invalidated object sort timestamps.
    SortTimestampsChanged,
}

impl CloudViewModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        ctx.subscribe_to_model(&CloudModel::handle(ctx), Self::handle_cloud_model_event);
        ctx.subscribe_to_model(
            &UpdateManager::handle(ctx),
            Self::handle_update_manager_event,
        );
        Self {
            folder_timestamp_cache: Default::default(),
        }
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(ctx)
    }

    fn handle_cloud_model_event(&mut self, event: &CloudModelEvent, ctx: &mut ModelContext<Self>) {
        match event {
            CloudModelEvent::ObjectUpdated { type_and_id, .. }
            | CloudModelEvent::ObjectTrashed { type_and_id, .. }
            | CloudModelEvent::ObjectUntrashed { type_and_id, .. }
            | CloudModelEvent::ObjectPermissionsUpdated { type_and_id, .. } => {
                // If an object is updated, we need to recompute the timestamps of its parents.
                if self.invalidate_object_timestamps(&type_and_id.uid(), CloudModel::as_ref(ctx)) {
                    ctx.emit(CloudViewModelEvent::SortTimestampsChanged);
                }
            }
            CloudModelEvent::ObjectMoved {
                from_folder,
                to_folder,
                ..
            } => {
                // Both the old parent and the new parent need to be invalidated, since this object
                // could affect the sort timestamp of both. Even if the moved object were a folder,
                // its own sort timestamp isn't affected.
                let cloud_model = CloudModel::as_ref(ctx);
                let old_parent_changed = from_folder.is_some_and(|folder_id| {
                    self.invalidate_folder_timestamps(&folder_id, cloud_model)
                });
                let new_parent_changed = to_folder.is_some_and(|folder_id| {
                    self.invalidate_folder_timestamps(&folder_id, cloud_model)
                });
                if old_parent_changed || new_parent_changed {
                    ctx.emit(CloudViewModelEvent::SortTimestampsChanged);
                }
            }
            CloudModelEvent::ObjectCreated { type_and_id } => {
                // There are three cases for an ObjectCreated event:
                // 1. We created a new object locally (in which case type_and_id is a client ID)
                // 2. We were notified about a new object from the server.
                // 3. A locally-created object was saved to the server, so we now have a server ID
                //    for it.
                // Because we sort on server timestamps, only the second or third cases can affect
                // sorting.
                if type_and_id.has_server_id()
                    && self
                        .invalidate_object_timestamps(&type_and_id.uid(), CloudModel::as_ref(ctx))
                {
                    ctx.emit(CloudViewModelEvent::SortTimestampsChanged);
                }
            }
            CloudModelEvent::ObjectDeleted { folder_id, .. } => {
                if let Some(folder_id) = folder_id {
                    if self.invalidate_folder_timestamps(folder_id, CloudModel::as_ref(ctx)) {
                        ctx.emit(CloudViewModelEvent::SortTimestampsChanged);
                    }
                }
            }
            CloudModelEvent::ObjectForceExpanded { .. }
            | CloudModelEvent::ObjectSynced { .. }
            | CloudModelEvent::InitialLoadCompleted => (),
        }
    }

    fn handle_update_manager_event(
        &mut self,
        event: &UpdateManagerEvent,
        _ctx: &mut ModelContext<Self>,
    ) {
        let result = event.result();

        if result.success_type != OperationSuccessType::Success {
            return;
        }

    }

    /// Invalidate all cached timestamps for the object with the given ID, and its parents.
    fn invalidate_object_timestamps(&mut self, uid: &ObjectUid, cloud_model: &CloudModel) -> bool {
        let Some(object) = cloud_model.get_by_uid(uid) else {
            return false;
        };
        if let Some(parent_id) = object.metadata().folder_id {
            self.invalidate_folder_timestamps(&parent_id, cloud_model)
        } else {
            false
        }
    }

    /// Invalidate all cached timestamps for the given folder and its parents.
    fn invalidate_folder_timestamps(
        &mut self,
        folder_id: &SyncId,
        _cloud_model: &CloudModel,
    ) -> bool {
        self.folder_timestamp_cache.borrow_mut().remove(folder_id).is_some()
    }
}

impl Entity for CloudViewModel {
    type Event = CloudViewModelEvent;
}

/// Mark CloudViewModel as global application state.
impl SingletonEntity for CloudViewModel {}

