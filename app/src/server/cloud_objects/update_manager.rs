use std::future::Future;
use std::sync::mpsc::SyncSender;
use chrono::Utc;

use warp_util::server_timestamp::ServerTimestamp;
use warp_util::sync::Condition;
use warpui::{
    Entity, ModelContext,
    SingletonEntity,
};

use crate::ai::mcp::templatable::{CloudTemplatableMCPServerModel, TemplatableMCPServer};
use crate::auth::AuthStateProvider;
use crate::cloud_object::model::actions::{
    ObjectActionType, ObjectActions,
};
use crate::cloud_object::model::persistence::CloudModel;
use crate::cloud_object::{
    CloudModelType, CloudObject, CloudObjectEventEntrypoint, CloudObjectLocation,
    GenericCloudObject, ObjectIdType, ObjectType, Owner,
    Revision,
};
use crate::drive::CloudObjectTypeAndId;
use crate::network::{NetworkStatus, NetworkStatusEvent, NetworkStatusKind};
use crate::persistence::ModelEvent;
use crate::server::ids::{
    ClientId, HashableId, ObjectUid, ServerId, SyncId,
    ToServerId,
};

use crate::workspaces::team_tester::{TeamTesterStatus, TeamTesterStatusEvent};
use crate::workspaces::user_workspaces::UserWorkspaces;


#[derive(Debug, PartialEq)]
pub enum OperationSuccessType {
    Success,
}

#[derive(Debug, PartialEq)]
pub enum ObjectOperation {
    MoveToFolder,
    MoveToDrive,
    Trash,
    Delete { initiated_by: InitiatedBy },
}

#[derive(Debug)]
pub struct ObjectOperationResult {
    pub success_type: OperationSuccessType,
    pub operation: ObjectOperation,
    pub client_id: Option<ClientId>,
    pub server_id: Option<ServerId>,
    pub num_objects: Option<i32>, // counts number of objects (including descendants) deleted for permadeletion
}

#[derive(Debug)]
pub enum UpdateManagerEvent {
    ObjectOperationComplete {
        result: ObjectOperationResult,
    },
}

impl UpdateManagerEvent {
    pub fn result(&self) -> &ObjectOperationResult {
        match self {
            UpdateManagerEvent::ObjectOperationComplete { result } => result,
        }
    }
}

/// An enum that defines whether the action was initiated by the user or the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitiatedBy {
    User,
    System,
}
/// The UpdateManager is responsible for delegating work
/// when there is an update to an object (e.g. via a user interaction or
/// a message from the server). Specifically, it will
/// - write to SQLite
/// - interact with the CloudModel to update the in-memory state used by the object views
pub struct UpdateManager {
    model_event_sender: Option<SyncSender<ModelEvent>>,
    has_initial_load: Condition,
}

impl UpdateManager {
    pub fn new(
        model_event_sender: Option<SyncSender<ModelEvent>>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let network_status = NetworkStatus::handle(ctx);
        ctx.subscribe_to_model(&network_status, |me, event, ctx| {
            me.handle_network_status_changed(event, ctx);
        });

        let team_tester_status = TeamTesterStatus::handle(ctx);
        ctx.subscribe_to_model(&team_tester_status, Self::handle_team_tester_status_changed);

        let manager = Self {
            model_event_sender,
            has_initial_load: Condition::new(),
        };
        // Drive server-sync amputated (local-only build): no server bulk-load fires,
        // so signal initial-load complete against the sqlite-populated CloudModel.
        manager.has_initial_load.set();
        manager
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(None, ctx)
    }

    fn save_to_db(&self, events: impl IntoIterator<Item = ModelEvent>) {
        let model_event_sender = self.model_event_sender.clone();
        if let Some(model_event_sender) = &model_event_sender {
            for event in events {
                if let Err(e) = model_event_sender.send(event) {
                    log::error!("Error saving to database: {e:?}");
                }
            }
        }
    }

    fn handle_team_tester_status_changed(
        &mut self,
        event: &TeamTesterStatusEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        let TeamTesterStatusEvent::InitiateDataPollers { force_refresh } = event;
        if *force_refresh {
            self.refresh_updated_objects(ctx);
        }

        self.start_polling_for_updated_objects(ctx);
    }


    pub fn start_polling_for_updated_objects(&mut self, _ctx: &mut ModelContext<Self>) {
        // Drive server-sync amputated (local-only build): no inbound polling.
    }

    /// Out-of-band (from the regular poll) refresh of updated objects.
    pub fn refresh_updated_objects(&mut self, _ctx: &mut ModelContext<Self>) {
        // Drive server-sync amputated (local-only build): no inbound refresh.
    }

    pub fn stop_polling_for_updated_objects(&mut self) {
        // Drive server-sync amputated (local-only build): no polling to stop.
    }

    fn handle_network_status_changed(
        &mut self,
        event: &NetworkStatusEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        match event {
            NetworkStatusEvent::NetworkStatusChanged { new_status } => match new_status {
                NetworkStatusKind::Online => {
                    self.start_polling_for_updated_objects(ctx);
                }
                NetworkStatusKind::Offline => {
                    self.stop_polling_for_updated_objects();
                }
            },
        }
    }

    /// Fetches environment "last used" timestamps from the server and merges them
    /// into the in-memory environment objects.
    /// Wait for an initial load to complete.
    pub fn initial_load_complete(&self) -> impl Future<Output = ()> {
        // We're not using `async fn` here so that the returned Future doesn't borrow self.
        self.has_initial_load.wait()
    }

    fn save_in_memory_object_to_sqlite(&mut self, cloud_model: &CloudModel, uid: &ObjectUid) {
        if let Some(cloud_object) = cloud_model.get_by_uid(uid) {
            self.save_to_db([cloud_object.upsert_event()]);
        }
    }

    pub fn update_templatable_mcp_server(
        &mut self,
        templatable_mcp_server: TemplatableMCPServer,
        templatable_mcp_server_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudTemplatableMCPServerModel::new(templatable_mcp_server),
            templatable_mcp_server_id,
            revision_ts,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn move_object_to_folder(
        &mut self,
        server_id: ServerId,
        _object_type: ObjectType,
        _owner: Owner,
        _destination_folder: Option<SyncId>,
        _current_folder: Option<SyncId>,
        _current_metadata_last_updated_ts: Option<ServerTimestamp>,
        ctx: &mut ModelContext<Self>,
    ) {
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(obj) = cloud_model.get_mut_by_uid(&server_id.uid()) {
                obj.metadata_mut()
                    .pending_changes_statuses
                    .has_pending_metadata_change = false;
            }
            ctx.notify();
        });
        self.save_in_memory_object_to_sqlite(CloudModel::as_ref(ctx), &server_id.uid());
        ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
            result: ObjectOperationResult {
                success_type: OperationSuccessType::Success,
                operation: ObjectOperation::MoveToFolder,
                client_id: None,
                server_id: Some(server_id),
                num_objects: None,
            },
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn move_object_to_drive(
        &mut self,
        server_id: ServerId,
        _object_type: ObjectType,
        _destination_owner: Owner,
        _current_folder: Option<SyncId>,
        _current_owner: Owner,
        _current_permissions_last_updated_ts: Option<ServerTimestamp>,
        ctx: &mut ModelContext<Self>,
    ) {
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(obj) = cloud_model.get_mut_by_uid(&server_id.uid()) {
                obj.metadata_mut()
                    .pending_changes_statuses
                    .has_pending_permissions_change = false;
            }
            ctx.notify();
        });
        self.save_in_memory_object_to_sqlite(CloudModel::as_ref(ctx), &server_id.uid());
        ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
            result: ObjectOperationResult {
                success_type: OperationSuccessType::Success,
                operation: ObjectOperation::MoveToDrive,
                client_id: None,
                server_id: Some(server_id),
                num_objects: None,
            },
        });
    }

    // This method moves an object from its current location to a new location.
    // Since moving is an online-only operation, this operation does NOT go through the sync queue.
    pub fn move_object_to_location(
        &mut self,
        object_id: CloudObjectTypeAndId,
        new_location: CloudObjectLocation,
        ctx: &mut ModelContext<Self>,
    ) {
        if let CloudObjectLocation::Trash = new_location {
            return;
        }

        // A move operation does not make sense offline,
        // so early return if we don't have a server ID for whatever reason.
        let uid = object_id.uid();
        let Some(server_id) = object_id.server_id() else {
            return;
        };

        let Some((
            object_current_owner,
            object_current_folder,
            object_type,
            has_pending_online_only_change,
            curr_metadata_ts,
            curr_permissions_ts,
        )) = CloudModel::handle(ctx).read(ctx, |model, _| {
            let object = model.get_by_uid(&uid)?;
            Some((
                object.permissions().owner,
                object.metadata().folder_id,
                object.into(),
                object.metadata().has_pending_online_only_change(),
                object.metadata().metadata_last_updated_ts,
                object.permissions().permissions_last_updated_ts,
            ))
        })
        else {
            return;
        };

        // We disallow stacked online-only changes so early return
        // if there's already one pending for this object.
        if has_pending_online_only_change {
            return;
        }

        // Apply a pending, optimistic update and then try to sync the move with the server.
        // We only update the in-memory data but don't persist anything in sqlite until the server confirms the move.
        // Todo: this logic shouldn't need to match based on Space versus Folder. Once we have moving across spaces in MoveObject,
        // we should simplify this to a unified call to move_object that sends the new space AND the new folder.
        let mut not_supported = false;
        match new_location {
            CloudObjectLocation::Space(destination_space) => {
                match UserWorkspaces::as_ref(ctx).space_to_owner(destination_space, ctx) {
                    Some(destination_owner) => {
                        if destination_owner == object_current_owner {
                            // If the space is staying the same, then the move must be to move to the root of the space.
                            CloudModel::handle(ctx).update(ctx, |model, ctx| {
                                model.update_object_location(&uid, None, None, ctx);
                            });
                            self.move_object_to_folder(
                                server_id,
                                object_type,
                                object_current_owner,
                                None,
                                object_current_folder,
                                curr_metadata_ts,
                                ctx,
                            );
                        } else {
                            CloudModel::handle(ctx).update(ctx, |model, ctx| {
                                model.update_object_location(
                                    &uid,
                                    Some(destination_owner),
                                    None,
                                    ctx,
                                );
                            });
                            self.move_object_to_drive(
                                server_id,
                                object_type,
                                destination_owner,
                                object_current_folder,
                                object_current_owner,
                                curr_permissions_ts,
                                ctx,
                            );
                        }
                    }
                    None => {
                        // We couldn't map the space to a valid owner (most likely, it's the
                        // "shared" space).
                        not_supported = true;
                    }
                }
            }
            CloudObjectLocation::Folder(SyncId::ServerId(destination_folder_id)) => {
                // If we're moving across folders, then the space must be staying the same.
                CloudModel::handle(ctx).update(ctx, |model, ctx| {
                    model.update_object_location(
                        &uid,
                        None,
                        Some(SyncId::ServerId(destination_folder_id)),
                        ctx,
                    );
                });
                self.move_object_to_folder(
                    server_id,
                    object_type,
                    object_current_owner,
                    Some(destination_folder_id.into()),
                    object_current_folder,
                    curr_metadata_ts,
                    ctx,
                );
            }
            _ => {
                not_supported = true;
            }
        }

        // In all other cases, just immediately revert the optimistic update since
        // we won't be trying to move the object and we don't want the object to appear
        // as pending.
        if not_supported {
            CloudModel::handle(ctx).update(ctx, |model, ctx| {
                model.update_object_location(
                    &uid,
                    Some(object_current_owner),
                    object_current_folder,
                    ctx,
                );
            });
        }

        ctx.notify();
    }

    pub fn create_templatable_mcp_server(
        &mut self,
        templatable_mcp_server: TemplatableMCPServer,
        client_id: ClientId,
        owner: Owner,
        initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudTemplatableMCPServerModel::new(templatable_mcp_server),
            owner,
            client_id,
            Default::default(),
            false,
            None,
            initiated_by,
            ctx,
        );
    }


    #[allow(clippy::too_many_arguments)]
    /// Bulk creates a list of generic string objects, all in a single
    /// sqllite write and server api call.  More efficient than calling
    /// create_object for each object.
    ///
    /// Note that if the bulk creation request fails, the client will end up retrying
    /// Generic function for creating a new cloud object with a given model.
    #[allow(clippy::too_many_arguments)]
    pub fn create_object<K, M>(
        &mut self,
        model: M,
        owner: Owner,
        client_id: ClientId,
        _entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        initial_folder_id: Option<SyncId>,
        _initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) where
        K: HashableId
            + ToServerId
            + std::fmt::Debug
            + Into<String>
            + Clone
            + Copy
            + Send
            + Sync
            + 'static,
        M: CloudModelType<IdType = K, CloudObjectType = GenericCloudObject<K, M>> + 'static,
    {
        let object_id = SyncId::ClientId(client_id);
        let auth_state = AuthStateProvider::as_ref(ctx).get();
        let initial_editor = auth_state.user_id();

        // Update in-memory model.
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            let mut object = GenericCloudObject::<K, M>::new_local(
                model.clone(),
                owner,
                initial_folder_id,
                client_id,
            );
            object.metadata.current_editor_uid = initial_editor.map(|uid| uid.as_string());
            cloud_model.create_object(object_id, object, ctx);

            if force_expand {
                cloud_model.force_expand_object_and_ancestors(object_id, ctx);
            }
        });

        // Update sqlite.
        let cloud_model = CloudModel::as_ref(ctx);
        if let Some(object) = cloud_model.get_object_of_type::<K, M>(&object_id) {
            self.save_to_db([object.upsert_event()]);
        }

    }


    /// Generic function for updating a cloud object with a new model.
    pub fn update_object<K, M>(
        &mut self,
        model: M,
        object_id: SyncId,
        _revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) where
        K: HashableId
            + ToServerId
            + std::fmt::Debug
            + Into<String>
            + Clone
            + Copy
            + Send
            + Sync
            + 'static,
        M: CloudModelType<IdType = K, CloudObjectType = GenericCloudObject<K, M>> + 'static,
    {
        // Update in-memory model.
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            cloud_model.update_object_from_edit(model.clone(), object_id, ctx);
            if let Some(object) = cloud_model.get_mut_by_uid(&object_id.uid()) {
                object.increment_in_flight_request_count();
                ctx.notify();
            }
        });

        // Update sqlite.
        let cloud_model = CloudModel::as_ref(ctx);
        if let Some(object) = cloud_model.get_object_of_type::<K, M>(&object_id) {
            self.save_to_db([object.upsert_event()]);
        };

    }

    // Takes a generic SyncId and records the action.
    pub fn record_object_action(
        &mut self,
        id_and_type: CloudObjectTypeAndId,
        action_type: ObjectActionType,
        data: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) {
        // Take the action timestamp from the client.
        let action_timestamp = Utc::now();

        // Update in-memory model.
        let object_action = ObjectActions::handle(ctx).update(ctx, |object_actions_model, ctx| {
            object_actions_model.insert_action(
                id_and_type.uid(),
                id_and_type.sqlite_uid_hash(),
                action_type.clone(),
                data.clone(),
                action_timestamp,
                ctx,
            )
        });

        // Update sqlite.
        self.save_to_db([ModelEvent::InsertObjectAction { object_action }]);

    }

    pub fn delete_object_by_user(
        &mut self,
        id: CloudObjectTypeAndId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.delete_object_with_initiated_by(id, InitiatedBy::User, ctx);
    }

    pub fn delete_object_with_initiated_by(
        &mut self,
        id: CloudObjectTypeAndId,
        initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) {
        let Some(server_id) = id.server_id() else {
            return;
        };

        let uid = id.uid();
        let Some((has_pending_online_only_operation, has_pending_delete)) = CloudModel::handle(ctx)
            .read(ctx, |model, _| {
                model.get_by_uid(&uid).map(|object| {
                    (
                        object.metadata().has_pending_online_only_change(),
                        object.metadata().pending_changes_statuses.pending_delete,
                    )
                })
            })
        else {
            return;
        };

        if has_pending_online_only_operation || has_pending_delete {
            return;
        }

        let num_deleted_objects = self.on_object_delete_success(vec![SyncId::ServerId(server_id)], ctx);
        ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
            result: ObjectOperationResult {
                success_type: OperationSuccessType::Success,
                operation: ObjectOperation::Delete { initiated_by },
                client_id: None,
                server_id: Some(ServerId::from_string_lossy(&uid)),
                num_objects: Some(num_deleted_objects),
            },
        });
        ctx.notify();
    }


    pub fn on_object_delete_success(
        &mut self,
        deleted_ids: Vec<SyncId>,
        ctx: &mut ModelContext<'_, UpdateManager>,
    ) -> i32 {
        let cloud_model_handle = CloudModel::handle(ctx);
        let all_object_uids: Vec<ObjectUid> = deleted_ids.iter().map(|&id| id.uid()).collect();

        // This variable counts the number of objects deleted client-side in each Empty Trash action,
        // because the server returns everything in the db, including objects that have already been marked for deletion
        let mut num_deleted_objects = 0;
        let mut sync_ids_and_types: Vec<(SyncId, ObjectIdType)> = Vec::new();
        cloud_model_handle.update(ctx, |cloud_model, ctx| {
            (sync_ids_and_types, num_deleted_objects) =
                cloud_model.delete_objects_by_id(all_object_uids.clone(), ctx);
        });

        // Deleted the actions associated with these objects too.
        ObjectActions::handle(ctx).update(ctx, |object_actions, ctx| {
            for uid in all_object_uids.clone() {
                object_actions.delete_actions_for_object(&uid, ctx);
            }
        });

        // Return early if empty
        if num_deleted_objects == 0 {
            return num_deleted_objects;
        }

        // Delete objects from sqlite. This will also delete their actions.
        self.save_to_db([ModelEvent::DeleteObjects {
            ids: sync_ids_and_types,
        }]);

        num_deleted_objects
    }

}

impl Entity for UpdateManager {
    type Event = UpdateManagerEvent;
}

impl SingletonEntity for UpdateManager {}

