use std::cell::RefCell;
use std::collections::HashMap;

use warp_util::server_timestamp::ServerTimestamp;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use super::persistence::{CloudModel, CloudModelEvent};
use crate::auth::AuthStateProvider;
use crate::cloud_object::{CloudObject, Space};
use crate::server::cloud_objects::update_manager::{
    OperationSuccessType, UpdateManager, UpdateManagerEvent,
};
use crate::server::ids::{ObjectUid, SyncId};
use warp_server_client::drive::sharing::SharingAccessLevel;

/// Whether or not a shared object's contents are editable by the current user.
///
/// Not purely a function of access level: anonymous users are not allowed to edit (lack of
/// attribution).
#[derive(Debug, Clone, Copy)]
pub enum ContentEditability {
    ReadOnly,
    RequiresLogin,
    Editable,
}

impl ContentEditability {
    pub fn can_edit(self) -> bool {
        matches!(self, ContentEditability::Editable)
    }
}

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

    fn object_access_level(object: &dyn CloudObject, app: &AppContext) -> SharingAccessLevel {
        match object.space(app) {
            // For now, users have full access to all objects in their own drives. We may introduce
            // drive-level ACLs in the future.
            Space::Personal => SharingAccessLevel::Full,
            Space::Shared => {
                let mut access_level = SharingAccessLevel::View;

                // Check the default link-based access (if set, this is *at least* View).
                if let Some(link_settings) = &object.permissions().anyone_with_link {
                    access_level = link_settings.access_level;
                }

                let user_uid = AuthStateProvider::as_ref(app).get().user_id();
                if let Some(user_uid) = user_uid {
                    for guest in object.permissions().guests.iter() {
                        if guest.subject.is_user(user_uid) {
                            access_level = access_level.max(guest.access_level);
                        }
                    }
                }

                // If the user created an object in a shared space, they will be treated as a guest and not the owner.
                // The guest permissions aren't fetched until the object is re-fetched, and this fixes this behavior
                // by forcing edit access if they created the object.
                if let (Some(creator_uid), Some(user_uid)) =
                    (object.metadata().creator_uid.clone(), user_uid)
                {
                    if creator_uid == user_uid.as_string() {
                        access_level = access_level.max(SharingAccessLevel::Edit);
                    }
                }

                access_level
            }
        }
    }

    /// Get the current user's editability state for a Warp Drive object.
    pub fn object_editability(
        &self,
        object_uid: &ObjectUid,
        app: &AppContext,
    ) -> ContentEditability {
        match CloudModel::as_ref(app).get_by_uid(object_uid) {
            Some(object) => {
                let access_level = Self::object_access_level(object, app);
                if access_level < SharingAccessLevel::Edit {
                    ContentEditability::ReadOnly
                } else if AuthStateProvider::as_ref(app)
                    .get()
                    .is_anonymous_or_logged_out()
                {
                    // The object is editable, but the user is not logged in.
                    if object.space(app) == Space::Personal {
                        ContentEditability::Editable
                    } else {
                        ContentEditability::RequiresLogin
                    }
                } else {
                    ContentEditability::Editable
                }
            }
            // Assume objects not yet in CloudModel are new, and therefore editable.
            None => ContentEditability::Editable,
        }
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

