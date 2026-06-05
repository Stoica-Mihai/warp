use std::collections::HashSet;
use std::future::Future;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

use chrono::Utc;
use futures::channel::oneshot::{self, Receiver};
use regex::Regex;
use warp_graphql::scalars::time::ServerTimestamp;
use warp_util::sync::Condition;
use warpui::{
    AppContext, Entity, ModelContext,
    SingletonEntity,
};

use crate::ai::facts::{AIFact, CloudAIFactModel};
#[cfg(not(target_family = "wasm"))]
use crate::ai::mcp::templatable::{CloudTemplatableMCPServerModel, TemplatableMCPServer};
use crate::auth::AuthStateProvider;
use crate::cloud_object::model::actions::{
    ObjectActionType, ObjectActions,
};
use crate::cloud_object::model::generic_string_model::{
    GenericStringObjectId,
};
use crate::cloud_object::model::persistence::{CloudModel, CloudModelEvent, UpdateSource};
use crate::cloud_object::model::view::{CloudViewModel, Editor, EditorState};
use crate::cloud_object::{
    CloudModelType, CloudObject, CloudObjectEventEntrypoint, CloudObjectLocation,
    GenericCloudObject,
    GenericStringObjectFormat, JsonObjectType, ObjectIdType, ObjectType, Owner,
    Revision, Space,
};
use crate::drive::folders::{CloudFolderModel, FolderId};
use crate::drive::CloudObjectTypeAndId;
use crate::env_vars::{CloudEnvVarCollectionModel, EnvVarCollection};
use crate::network::{NetworkStatus, NetworkStatusEvent, NetworkStatusKind};
use crate::notebooks::{CloudNotebookModel, NotebookId};
use crate::persistence::ModelEvent;
use crate::server::ids::{
    ClientId, HashableId, ObjectUid, ServerId, SyncId,
    ToServerId,
};
use crate::workflows::workflow::Workflow;
use crate::workflows::workflow_enum::{CloudWorkflowEnum, CloudWorkflowEnumModel, WorkflowEnum};
use crate::workflows::{CloudWorkflowModel, WorkflowId};
use crate::workspaces::team_tester::{TeamTesterStatus, TeamTesterStatusEvent};
use crate::workspaces::user_workspaces::UserWorkspaces;

lazy_static::lazy_static! {
    static ref DUPLICATE_OBJECT_NAME_REGEX: Regex = Regex::new(r" \((\d+)\)$").expect("regex should not fail to compile");
}

#[derive(Debug, PartialEq)]
pub enum OperationSuccessType {
    Success,
    Rejection,
}

#[derive(Debug, PartialEq)]
pub enum ObjectOperation {
    Create { initiated_by: InitiatedBy },
    Update,
    MoveToFolder,
    MoveToDrive,
    Trash,
    TakeEditAccess,
    Untrash,
    Delete { initiated_by: InitiatedBy },
    EmptyTrash,
    Leave,
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

/// An enum for choosing the behavior of the fetch_single_cloud_object function.
pub enum FetchSingleObjectOption {
    /// Perform the normal upsert behavior.
    None,
    /// Perform the normal upsert behavior, but additionally force overwrite the
    /// in-memory object to whatever the server object is.
    ForceOverwrite,
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

    pub fn resync_object(
        &mut self,
        _cloud_object_type_and_id: &CloudObjectTypeAndId,
        _ctx: &mut ModelContext<Self>,
    ) {
        // Local-only: no server to re-sync with.
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

    fn save_in_memory_object_metadata_to_sqlite(
        &mut self,
        cloud_model: &CloudModel,
        uid: &ObjectUid,
        hashed_sqlite_id: &str,
    ) {
        if let Some(cloud_object) = cloud_model.get_by_uid(uid) {
            let metadata = cloud_object.metadata().clone();
            let event = ModelEvent::UpdateObjectMetadata {
                id: hashed_sqlite_id.to_string(),
                metadata,
            };
            self.save_to_db([event]);
        }
    }

    /// Local-only: no server to fetch from; signals completion immediately.
    pub fn fetch_single_cloud_object(
        &mut self,
        _server_id: &ServerId,
        _fetch_single_object_option: FetchSingleObjectOption,
        _ctx: &mut ModelContext<Self>,
    ) -> Receiver<()> {
        let (tx, rx) = oneshot::channel::<()>();
        let _ = tx.send(());
        rx
    }

    /// Replace an object's data with the conflicting version from the server. If the object does
    /// not have a conflict, this has no effect.
    pub fn replace_object_with_conflict(&mut self, uid: &ObjectUid, ctx: &mut ModelContext<Self>) {
        let cloud_model_handle = CloudModel::handle(ctx);

        // Update the in-memory model first, and check for conflicts.
        let had_conflicts = cloud_model_handle.update(ctx, |cloud_model, ctx| {
            match cloud_model.get_mut_by_uid(uid) {
                Some(object) if object.has_conflicting_changes() => {
                    object.replace_object_with_conflict();
                    ctx.emit(CloudModelEvent::ObjectUpdated {
                        type_and_id: object.cloud_object_type_and_id(),
                        source: UpdateSource::Server,
                    });
                    true
                }
                _ => false,
            }
        });

        // Update SQLite, but only if the in-memory model was updated.
        if had_conflicts {
            self.save_in_memory_object_to_sqlite(cloud_model_handle.as_ref(ctx), uid);
        }
    }

    pub fn update_ai_fact(
        &mut self,
        ai_fact: AIFact,
        ai_fact_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(CloudAIFactModel::new(ai_fact), ai_fact_id, revision_ts, ctx);
    }

    #[cfg(not(target_family = "wasm"))]
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

    pub fn update_workflow(
        &mut self,
        workflow: Workflow,
        workflow_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudWorkflowModel::new(workflow),
            workflow_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_workflow_enum(
        &mut self,
        workflow_enum: WorkflowEnum,
        workflow_enum_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudWorkflowEnumModel::new(workflow_enum),
            workflow_enum_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_env_var_collection(
        &mut self,
        env_var_collection: EnvVarCollection,
        env_var_collection_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudEnvVarCollectionModel::new(env_var_collection),
            env_var_collection_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_notebook_data(
        &mut self,
        data: Arc<String>,
        notebook_id: SyncId,
        ctx: &mut ModelContext<Self>,
    ) {
        let cloud_model = CloudModel::as_ref(ctx);
        let revision = cloud_model.current_revision(&notebook_id).cloned();
        if let Some(notebook) = cloud_model.get_notebook(&notebook_id) {
            let new_notebook = CloudNotebookModel {
                title: notebook.model().title.to_owned(),
                data: data.to_string(),
                ai_document_id: notebook.model().ai_document_id,
                conversation_id: notebook.model().conversation_id.clone(),
            };
            self.update_object(new_notebook, notebook_id, revision, ctx);
        } else {
            log::warn!("Expected notebook to be in model with id {notebook_id:?}");
        }
    }

    pub fn update_notebook_title(
        &mut self,
        title: Arc<String>,
        notebook_id: SyncId,
        ctx: &mut ModelContext<Self>,
    ) {
        let cloud_model = CloudModel::as_ref(ctx);
        let revision = cloud_model.current_revision(&notebook_id).cloned();
        if let Some(notebook) = cloud_model.get_notebook(&notebook_id) {
            let new_notebook = CloudNotebookModel {
                title: title.to_string(),
                data: notebook.model().data.to_owned(),
                ai_document_id: notebook.model().ai_document_id,
                conversation_id: notebook.model().conversation_id.clone(),
            };
            self.update_object(new_notebook, notebook_id, revision, ctx);
        } else {
            log::warn!("Expected notebook to be in model with id {notebook_id:?}");
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn move_object_to_folder(
        &mut self,
        server_id: ServerId,
        _object_type: ObjectType,
        _owner: Owner,
        _destination_folder: Option<FolderId>,
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
        object_type: ObjectType,
        destination_owner: Owner,
        _current_folder: Option<SyncId>,
        _current_owner: Owner,
        _current_permissions_last_updated_ts: Option<ServerTimestamp>,
        ctx: &mut ModelContext<Self>,
    ) {
        if object_type == ObjectType::Workflow {
            self.copy_workflow_enums_to_drive(server_id, destination_owner, ctx);
        }

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

    /// Leaves a shared object. Local-only: no server; emit Success immediately.
    pub fn leave_object(&mut self, server_id: ServerId, ctx: &mut ModelContext<Self>) {
        let uid = server_id.uid();

        if CloudModel::as_ref(ctx)
            .get_by_uid(&uid)
            .is_none_or(|object| object.metadata().has_pending_online_only_change())
        {
            return;
        }

        let deleted_objects = CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            cloud_model.delete_object_and_descendants(uid.clone(), ctx)
        });

        ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
            result: ObjectOperationResult {
                success_type: OperationSuccessType::Success,
                operation: ObjectOperation::Leave,
                client_id: None,
                server_id: Some(server_id),
                num_objects: Some(deleted_objects.len() as i32),
            },
        });

        ObjectActions::handle(ctx).update(ctx, |object_actions, ctx| {
            for (id, _) in deleted_objects.iter() {
                object_actions.delete_actions_for_object(&id.uid(), ctx);
            }
        });

        self.save_to_db([ModelEvent::DeleteObjects { ids: deleted_objects }]);
    }

    /// Given a workflow_id and a destination drive, make a copy of all referenced workflow enums in the destination drive.
    /// Returns the original workflow object if it was modified (in case a future revert is needed), otherwise returns None.
    fn copy_workflow_enums_to_drive(
        &mut self,
        server_id: ServerId,
        owner: Owner,
        ctx: &mut ModelContext<Self>,
    ) -> Option<Workflow> {
        let workflow_id = SyncId::ServerId(server_id);
        let workflow = CloudModel::as_ref(ctx).get_workflow(&workflow_id);

        if let Some(workflow) = workflow {
            let original_workflow = workflow.model().data.clone();
            let mut workflow_model = original_workflow.clone();

            // Duplicate all enums associated with the workflow
            let enums = workflow_model.get_enum_ids();
            for enum_id in enums.iter() {
                let cloud_model = CloudModel::as_ref(ctx);
                let object: Option<&CloudWorkflowEnum> = cloud_model.get_object_of_type(enum_id);
                let Some(object) = object else {
                    log::error!("Could not find referenced workflow enum to copy over to the new space, skipping");
                    continue;
                };

                let client_id = ClientId::new();

                // Create a duplicate enum in the new space with a new client ID
                self.create_object(
                    object.model().clone(),
                    owner,
                    client_id,
                    CloudObjectEventEntrypoint::Unknown,
                    true,
                    None,
                    // When adding the initiated_by parameter to this function call, InitiatedBy::User was set as a default value.
                    // This can be changed to InitiatedBy::System if this action was automatically kicked off by the system and we do not want a user facing toast.
                    InitiatedBy::User,
                    ctx,
                );

                workflow_model.replace_object_id(*enum_id, SyncId::ClientId(client_id));
            }

            // Update the workflow with the new enum IDs, if there are any
            if !enums.is_empty() {
                self.update_workflow(workflow_model, workflow_id, None, ctx);
                Some(original_workflow)
            } else {
                None
            }
        } else {
            log::error!(
                "Tried to move workflow enums to new space but could not find associated workflow",
            );
            None
        }
    }

    // This method moves an object from its current location to a new location.
    // Since moving is an online-only operation, this operation does NOT go through the sync queue.
    pub fn move_object_to_location(
        &mut self,
        object_id: CloudObjectTypeAndId,
        new_location: CloudObjectLocation,
        ctx: &mut ModelContext<Self>,
    ) {
        // If we are moving into the trash, we really mean to trash the object
        if let CloudObjectLocation::Trash = new_location {
            return self.trash_object(object_id, ctx);
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

    pub fn duplicate_object(
        &mut self,
        cloud_object_type_and_id: &CloudObjectTypeAndId,
        ctx: &mut ModelContext<Self>,
    ) {
        match cloud_object_type_and_id {
            CloudObjectTypeAndId::Notebook(notebook_id) => {
                self.duplicate_object_internal::<NotebookId, CloudNotebookModel>(notebook_id, ctx);
            }
            CloudObjectTypeAndId::Workflow(workflow_id) => {
                self.duplicate_object_internal::<WorkflowId, CloudWorkflowModel>(workflow_id, ctx);
            }
            CloudObjectTypeAndId::GenericStringObject { object_type, id } => {
                if let GenericStringObjectFormat::Json(JsonObjectType::EnvVarCollection) =
                    object_type
                {
                    self.duplicate_object_internal::<GenericStringObjectId, CloudEnvVarCollectionModel>(
                        id, ctx,
                    );
                } else {
                    log::error!("Tried to duplicate an unsupported type: json object");
                    debug_assert!(false, "Tried to duplicate an unsupported type: json object");
                }
            }
            CloudObjectTypeAndId::Folder(_) => {
                // Duplicating folders not currently supported.
                log::error!("Tried to duplicate an unsupported type: folder");
                debug_assert!(false, "Tried to duplicate an unsupported type: folder");
            }
        }
    }

    fn duplicate_object_internal<K, M>(&mut self, id: &SyncId, ctx: &mut ModelContext<Self>)
    where
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
        let (duplicate_model, client_id, owner, initial_folder_id, entrypoint) = {
            let cloud_model = CloudModel::as_ref(ctx);
            let object: GenericCloudObject<K, M> = cloud_model
                .get_object_of_type(id)
                .expect("object should exist in order to be duplicated")
                .clone();
            let client_id = ClientId::new();
            let owner = object.permissions.owner;
            let initial_folder_id = object.metadata.folder_id;
            let entrypoint = CloudObjectEventEntrypoint::Unknown;
            let mut duplicate_model = object.model().clone();
            let duplicate_name =
                self.get_next_duplicate_object_name(&object as &dyn CloudObject, cloud_model, ctx);
            duplicate_model.set_display_name(&duplicate_name);
            (
                duplicate_model,
                client_id,
                owner,
                initial_folder_id,
                entrypoint,
            )
        };
        self.create_object(
            duplicate_model,
            owner,
            client_id,
            entrypoint,
            true,
            initial_folder_id,
            // When adding the initiated_by parameter to this function call, InitiatedBy::User was set as a default value.
            // This can be changed to InitiatedBy::System if this action was automatically kicked off by the system and we do not want a user facing toast.
            InitiatedBy::User,
            ctx,
        );
    }

    pub fn create_ai_fact(
        &mut self,
        ai_fact: AIFact,
        client_id: ClientId,
        owner: Owner,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudAIFactModel::new(ai_fact),
            owner,
            client_id,
            Default::default(),
            false,
            None,
            // When adding the initiated_by parameter to this function call, InitiatedBy::User was set as a default value.
            // This can be changed to InitiatedBy::System if this action was automatically kicked off by the system and we do not want a user facing toast.
            InitiatedBy::User,
            ctx,
        );
    }

    #[cfg(not(target_family = "wasm"))]
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
    pub fn create_notebook(
        &mut self,
        client_id: ClientId,
        owner: Owner,
        initial_folder_id: Option<SyncId>,
        model: CloudNotebookModel,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            model,
            owner,
            client_id,
            entrypoint,
            force_expand,
            initial_folder_id,
            // When adding the initiated_by parameter to this function call, InitiatedBy::User was set as a default value.
            // This can be changed to InitiatedBy::System if this action was automatically kicked off by the system and we do not want a user facing toast.
            InitiatedBy::User,
            ctx,
        );
    }

    fn get_next_duplicate_object_name(
        &self,
        original_cloud_object: &dyn CloudObject,
        cloud_model: &CloudModel,
        app: &AppContext,
    ) -> String {
        let original_name = original_cloud_object.display_name();

        // Iterate through items in the same folder as the original object that are of the
        // same type, and populate a hashset with those names.
        let same_type_and_folder_names = cloud_model
            .active_cloud_objects_in_location_without_descendents(
                original_cloud_object.location(cloud_model, app),
                app,
            )
            .filter(|&object| object.object_type() == original_cloud_object.object_type())
            .map(|object| object.display_name())
            .collect::<HashSet<String>>();

        // Start with "{original_object_name} ({original_object_name's count + 1})".
        // Keep incrementing by one if there already exists an object of the same type in
        // the same folder (using the hashset generated above).
        let mut duplicate_name = get_duplicate_object_name(&original_name);
        while same_type_and_folder_names.contains(&duplicate_name) {
            duplicate_name = get_duplicate_object_name(&duplicate_name);
        }
        duplicate_name
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_workflow(
        &mut self,
        workflow: Workflow,
        owner: Owner,
        initial_folder_id: Option<SyncId>,
        client_id: ClientId,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudWorkflowModel::new(workflow),
            owner,
            client_id,
            entrypoint,
            force_expand,
            initial_folder_id,
            // When adding the initiated_by parameter to this function call, InitiatedBy::User was set as a default value.
            // This can be changed to InitiatedBy::System if this action was automatically kicked off by the system and we do not want a user facing toast.
            InitiatedBy::User,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_workflow_enum(
        &mut self,
        workflow_enum: WorkflowEnum,
        owner: Owner,
        client_id: ClientId,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudWorkflowEnumModel::new(workflow_enum),
            owner,
            client_id,
            entrypoint,
            force_expand,
            None,
            // When adding the initiated_by parameter to this function call, InitiatedBy::User was set as a default value.
            // This can be changed to InitiatedBy::System if this action was automatically kicked off by the system and we do not want a user facing toast.
            InitiatedBy::User,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_env_var_collection(
        &mut self,
        client_id: ClientId,
        owner: Owner,
        initial_folder_id: Option<SyncId>,
        model: CloudEnvVarCollectionModel,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            model,
            owner,
            client_id,
            entrypoint,
            force_expand,
            initial_folder_id,
            // When adding the initiated_by parameter to this function call, InitiatedBy::User was set as a default value.
            // This can be changed to InitiatedBy::System if this action was automatically kicked off by the system and we do not want a user facing toast.
            InitiatedBy::User,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_folder(
        &mut self,
        name: String,
        owner: Owner,
        client_id: ClientId,
        initial_folder_id: Option<SyncId>,
        force_expand: bool,
        initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            // TODO(INT-789): support creating folders as warp packs
            CloudFolderModel::new(&name, false),
            owner,
            client_id,
            Default::default(),
            force_expand,
            initial_folder_id,
            initiated_by,
            ctx,
        );
    }

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

    /// Sets the notebooks current editor in memory. SQLite is not updated until we receive
    /// server confirmation.
    fn set_notebook_current_editor(
        &self,
        notebook_id: &SyncId,
        editor_uid: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) {
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(notebook) = cloud_model.get_notebook_mut(notebook_id) {
                notebook.metadata.set_current_editor(editor_uid);
                ctx.notify();
            }
        });
    }

    pub fn grab_notebook_edit_access(
        &mut self,
        notebook_id: SyncId,
        optimistically_grant_access: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        let SyncId::ServerId(server_id) = notebook_id else {
            return;
        };

        let auth_state = AuthStateProvider::as_ref(ctx).get();
        let user_uid = auth_state.user_id().unwrap_or_default();
        self.set_notebook_current_editor(&notebook_id, Some(user_uid.as_string()), ctx);
        if !optimistically_grant_access {
            ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
                result: ObjectOperationResult {
                    success_type: OperationSuccessType::Success,
                    operation: ObjectOperation::TakeEditAccess,
                    client_id: None,
                    server_id: Some(server_id),
                    num_objects: None,
                },
            });
        }
    }

    pub fn give_up_notebook_edit_access(
        &mut self,
        notebook_id: SyncId,
        ctx: &mut ModelContext<Self>,
    ) {
        let SyncId::ServerId(_server_id) = notebook_id else {
            return;
        };

        let current_editor = CloudViewModel::as_ref(ctx)
            .object_current_editor(&notebook_id.uid(), ctx)
            .unwrap_or(Editor::no_editor());

        if matches!(current_editor.state, EditorState::CurrentUser) {
            self.set_notebook_current_editor(&notebook_id, None, ctx);
        }
    }

    /// Optimistically marks the object as trashed, updates the metadata sync status to pending, and returns both
    /// the metadata timestamp and the newly-set trashed timestamp. We need to check the metadata timestamp
    /// in the case where we need to revert this (i.e. if there was a rtc message in the meantime, we shouldn't
    /// overwrite the values and don't need to).
    // TODO: we currently set trashed_ts here with the client's clock, but we should revise this metadata flow
    // to get the timestamp from the server instead.
    fn mark_object_trashed_and_return_timestamps(
        &self,
        uid: &ObjectUid,
        ctx: &mut ModelContext<Self>,
    ) -> (Option<ServerTimestamp>, Option<ServerTimestamp>) {
        let timestamp = ServerTimestamp::new(Utc::now());
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(object) = cloud_model.get_mut_by_uid(uid) {
                // Here, we write a timestamp to the trashed_ts field. The client will eventually update to
                // the canonical version of the timestamp once it receives an rtc message from the server.

                object.metadata_mut().trashed_ts = Some(timestamp);
                object
                    .metadata_mut()
                    .pending_changes_statuses
                    .has_pending_metadata_change = true;
                ctx.emit(CloudModelEvent::ObjectTrashed {
                    type_and_id: object.cloud_object_type_and_id(),
                    source: UpdateSource::Local,
                });
                ctx.notify();
                (
                    object.metadata().metadata_last_updated_ts,
                    object.metadata().trashed_ts,
                )
            } else {
                (None, None)
            }
        })
    }

    pub fn trash_object(&mut self, id: CloudObjectTypeAndId, ctx: &mut ModelContext<Self>) {
        let Some(server_id) = id.server_id() else {
            return;
        };

        let hashed_id = id.uid();
        let Some(has_pending_online_only_operation) =
            CloudModel::handle(ctx).read(ctx, |model, _| {
                model
                    .get_by_uid(&hashed_id)
                    .map(|object| object.metadata().has_pending_online_only_change())
            })
        else {
            return;
        };

        if has_pending_online_only_operation {
            return;
        }

        self.mark_object_trashed_and_return_timestamps(&hashed_id, ctx);

        CloudModel::handle(ctx).update(ctx, |cloud_model, _| {
            if let Some(object) = cloud_model.get_mut_by_uid(&hashed_id) {
                object
                    .metadata_mut()
                    .pending_changes_statuses
                    .has_pending_metadata_change = false;
            }
        });

        let hashed_sqlite_id = server_id.sqlite_type_and_uid_hash(id.object_id_type());
        let cloud_model = CloudModel::as_ref(ctx);
        self.save_in_memory_object_metadata_to_sqlite(cloud_model, &hashed_id, &hashed_sqlite_id);

        ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
            result: ObjectOperationResult {
                success_type: OperationSuccessType::Success,
                operation: ObjectOperation::Trash,
                client_id: None,
                server_id: Some(ServerId::from_string_lossy(&hashed_id)),
                num_objects: None,
            },
        });
        ctx.notify();
    }

    pub fn untrash_object(&mut self, id: CloudObjectTypeAndId, ctx: &mut ModelContext<Self>) {
        let Some(_server_id) = id.server_id() else {
            return;
        };

        let hashed_id = id.uid();
        let Some(has_pending_online_only_operation) =
            CloudModel::handle(ctx).read(ctx, |model, _| {
                model
                    .get_by_uid(&hashed_id)
                    .map(|object| object.metadata().has_pending_online_only_change())
            })
        else {
            return;
        };

        if has_pending_online_only_operation {
            return;
        }

        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(object) = cloud_model.get_mut_by_uid(&hashed_id) {
                object.metadata_mut().trashed_ts = None;
                object
                    .metadata_mut()
                    .pending_changes_statuses
                    .pending_untrash = false;
                ctx.emit(CloudModelEvent::ObjectUntrashed {
                    type_and_id: object.cloud_object_type_and_id(),
                    source: UpdateSource::Local,
                });
                ctx.notify();
            }
        });

        ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
            result: ObjectOperationResult {
                success_type: OperationSuccessType::Success,
                operation: ObjectOperation::Untrash,
                client_id: None,
                server_id: Some(ServerId::from_string_lossy(&hashed_id)),
                num_objects: None,
            },
        });
        ctx.notify();
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

    pub fn empty_trash(&mut self, space: Space, ctx: &mut ModelContext<Self>) {
        let owner = match UserWorkspaces::as_ref(ctx).space_to_owner(space, ctx) {
            Some(owner) => owner,
            None => {
                log::warn!("Tried to empty trash in unsupported space {space:?}");
                return;
            }
        };

        let trashed_ids: Vec<SyncId> = {
            let cloud_model = CloudModel::as_ref(ctx);
            cloud_model
                .get_all_exportable_object_ids()
                .into_iter()
                .filter_map(|type_and_id| {
                    let server_id = type_and_id.server_id()?;
                    let uid = type_and_id.uid();
                    let obj = cloud_model.get_by_uid(&uid)?;
                    if obj.metadata().trashed_ts.is_some() && obj.permissions().owner == owner {
                        Some(SyncId::ServerId(server_id))
                    } else {
                        None
                    }
                })
                .collect()
        };

        let num_deleted_objects = self.on_object_delete_success(trashed_ids, ctx);

        if num_deleted_objects == 0 {
            ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
                result: ObjectOperationResult {
                    success_type: OperationSuccessType::Rejection,
                    operation: ObjectOperation::EmptyTrash,
                    client_id: None,
                    server_id: None,
                    num_objects: Some(0),
                },
            });
        } else {
            ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
                result: ObjectOperationResult {
                    success_type: OperationSuccessType::Success,
                    operation: ObjectOperation::EmptyTrash,
                    client_id: None,
                    server_id: None,
                    num_objects: Some(num_deleted_objects),
                },
            });
        }
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

    pub fn rename_folder(
        &mut self,
        folder_id: SyncId,
        new_name: String,
        ctx: &mut ModelContext<Self>,
    ) {
        let cloud_model = CloudModel::as_ref(ctx);
        let revision = cloud_model.current_revision(&folder_id).cloned();
        if let Some(folder) = cloud_model.get_folder(&folder_id) {
            let new_folder = CloudFolderModel {
                name: new_name,
                is_open: folder.model().is_open,
                is_warp_pack: folder.model().is_warp_pack,
            };
            self.update_object(new_folder, folder_id, revision, ctx);
        } else {
            log::warn!("Attempted to rename folder that doesn't exist with id: {folder_id:?}");
        }
    }

}

/// Return the newly duplicated object's name based on the original object's name. E.g.:
/// - "my object name" -> "my object name (1)"
pub fn get_duplicate_object_name(original_name: &str) -> String {
    match DUPLICATE_OBJECT_NAME_REGEX
        .captures(original_name)
        .and_then(|caps| caps.get(1))
        .and_then(|num| num.as_str().parse::<usize>().ok())
    {
        Some(num) => {
            let new_num = num.saturating_add(1);

            // edge case check for when the duplicate number is usize::MAX
            if new_num == usize::MAX {
                format!("{original_name} (1)")
            } else {
                DUPLICATE_OBJECT_NAME_REGEX
                    .replace(original_name, format!(" ({new_num})"))
                    .to_string()
            }
        }
        None => format!("{original_name} (1)"),
    }
}

impl Entity for UpdateManager {
    type Event = UpdateManagerEvent;
}

impl SingletonEntity for UpdateManager {}

