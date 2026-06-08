use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::channel::oneshot;
use parking_lot::Mutex;
use tempfile::TempDir;
use warp_util::path::ShellFamily;
use warpui::{AddSingletonModel, App, SingletonEntity, WindowId};

use super::{safe_filename, ExportEvent, ExportId, ExportManager};
use crate::cloud_object::model::persistence::CloudModel;
use crate::cloud_object::{
    CloudObjectMetadata, CloudObjectPermissions, ObjectType, Space,
};
use crate::drive::CloudObjectTypeAndId;
use crate::server::ids::SyncId;
use crate::workflows::workflow::Workflow;
use crate::workflows::{CloudWorkflow, CloudWorkflowModel, WorkflowId};
use crate::workspace::ToastStack;
use crate::workspaces::user_workspaces::UserWorkspaces;

struct ExportTest {
    target_dir: TempDir,
    pending_exports: Arc<Mutex<HashMap<ExportId, oneshot::Sender<ExportEvent>>>>,
}

impl ExportTest {
    fn new(app: &mut App) -> Self {
        let pending_exports = Arc::new(Mutex::new(
            HashMap::<ExportId, oneshot::Sender<ExportEvent>>::new(),
        ));

        {
            let pending_exports = pending_exports.clone();
            app.update(|ctx| {
                ctx.subscribe_to_model(&ExportManager::handle(ctx), move |_, event, _| {
                    let mut pending_exports = pending_exports.lock();
                    let id = match event {
                        ExportEvent::Canceled(id) => id,
                        ExportEvent::Failed { id, .. } => id,
                        ExportEvent::Completed { id, .. } => id,
                    };
                    if let Some(sender) = pending_exports.remove(id) {
                        let _ = sender.send(event.clone());
                    }
                });
            });
        }

        Self {
            target_dir: TempDir::new().expect("failed to create temporary export directory"),
            pending_exports,
        }
    }

    /// Starts exporting an object into the temporary directory.
    fn start_export(
        &self,
        export_ids: CloudObjectTypeAndId,
        app: &mut App,
    ) -> (ExportId, oneshot::Receiver<ExportEvent>) {
        let id = ExportId(export_ids, Space::Personal);
        let (tx, rx) = oneshot::channel();
        self.pending_exports.lock().insert(id, tx);

        ExportManager::handle(app).update(app, |export_manager, ctx| {
            let window_id = WindowId::new();
            export_manager.export(window_id, &[export_ids], ctx);
            export_manager.handle_files_picked(
                vec![id],
                Ok(vec![self
                    .target_dir
                    .path()
                    .to_str()
                    .expect("Path must be UTF-8")
                    .to_owned()]),
                ShellFamily::Posix,
                ctx,
            );
            id
        });

        (id, rx)
    }

    /// Get an export path, given the expected name.
    fn path(&self, name: impl AsRef<Path>, space: Option<Space>, app: &App) -> PathBuf {
        if let Some(space) = space {
            let space_name = app.read(|ctx| space.name(ctx));
            self.target_dir.path().join(space_name).join(name)
        } else {
            self.target_dir.path().join(name)
        }
    }
}

fn initialize_app(app: &mut App) {
    app.add_singleton_model(CloudModel::mock);
    app.add_singleton_model(ExportManager::new);
    app.add_singleton_model(UserWorkspaces::default_mock);
    app.add_singleton_model(|_| ToastStack);
}

/// Add a mocked workflow.
fn add_workflow(id: SyncId, workflow: Workflow, app: &mut App) {
    CloudModel::handle(app).update(app, |cloud_model, _ctx| {
        cloud_model.add_object(
            id,
            CloudWorkflow::new(
                id,
                CloudWorkflowModel::new(workflow),
                CloudObjectMetadata::mock(),
                CloudObjectPermissions::mock_personal(),
            ),
        );
    });
}

#[test]
fn test_export_workflow_success() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        let workflow_id = SyncId::ServerId(WorkflowId::from(123).into());
        let workflow = Workflow::new("Test workflow", "echo hello world");
        add_workflow(workflow_id, workflow, &mut app);

        let exporter = ExportTest::new(&mut app);
        let (id, export) = exporter.start_export(
            CloudObjectTypeAndId::from_id_and_type(workflow_id, ObjectType::Workflow),
            &mut app,
        );
        let expected_path = exporter.path("Test workflow.yaml", None, &app);

        // The export should succeed.
        assert_eq!(
            export.await,
            Ok(ExportEvent::Completed {
                id,
                path: expected_path.clone()
            })
        );

        let contents =
            fs::read_to_string(&expected_path).expect("failed to read exported workflow");
        assert_eq!(
            &contents,
            r#"---
name: Test workflow
command: echo hello world
description: ~
arguments: []
tags: []
source_url: ~
author: ~
author_url: ~
shells: []
"#
        );
    });
}

#[test]
fn test_export_workflow_duplicate() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        let workflow_id = SyncId::ServerId(WorkflowId::from(123).into());
        let workflow = Workflow::new("Test workflow", "echo hello world");
        add_workflow(workflow_id, workflow, &mut app);

        let exporter = ExportTest::new(&mut app);

        // Create a file at the default export path.
        fs::write(
            exporter.path("Test workflow.yaml", None, &app),
            "Already exists",
        )
        .expect("failed to write existing workflow");

        let (id, export) = exporter.start_export(
            CloudObjectTypeAndId::from_id_and_type(workflow_id, ObjectType::Workflow),
            &mut app,
        );
        let expected_path = exporter.path("Test workflow (1).yaml", None, &app);

        // The export should succeed, and not overwrite the existing file.
        assert_eq!(
            export.await,
            Ok(ExportEvent::Completed {
                id,
                path: expected_path.clone()
            })
        );

        assert_eq!(
            fs::read_to_string(exporter.path("Test workflow.yaml", None, &app))
                .expect("failed to read original file"),
            "Already exists"
        );

        let contents =
            fs::read_to_string(&expected_path).expect("failed to read exported workflow");
        assert_eq!(
            &contents,
            r#"---
name: Test workflow
command: echo hello world
description: ~
arguments: []
tags: []
source_url: ~
author: ~
author_url: ~
shells: []
"#
        );
    });
}

#[test]
fn test_export_workflow_failure() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        let workflow_id = SyncId::ServerId(WorkflowId::from(123).into());
        let workflow = Workflow::new("Test workflow", "echo hello world");
        add_workflow(workflow_id, workflow, &mut app);

        let exporter = ExportTest::new(&mut app);
        // Ensure that the export will fail.
        fs::remove_dir_all(exporter.target_dir.path()).expect("Could not remove test directory");

        let (id, export) = exporter.start_export(
            CloudObjectTypeAndId::from_id_and_type(workflow_id, ObjectType::Workflow),
            &mut app,
        );

        // The export should error.
        assert_eq!(export.await, Ok(ExportEvent::Failed { id }));
    });
}

#[test]
fn test_export_with_special_characters() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        let workflow_id = SyncId::ServerId(WorkflowId::from(123).into());
        let workflow = Workflow::new("Prefix: Some/workflow", "echo hello world");
        add_workflow(workflow_id, workflow, &mut app);

        let exporter = ExportTest::new(&mut app);
        let (id, export) = exporter.start_export(
            CloudObjectTypeAndId::from_id_and_type(workflow_id, ObjectType::Workflow),
            &mut app,
        );
        let expected_path = exporter.path("Prefix_ Some_workflow.yaml", None, &app);

        // The export should succeed, and transform the path.
        assert_eq!(
            export.await,
            Ok(ExportEvent::Completed {
                id,
                path: expected_path.clone()
            })
        );
    });
}

#[test]
fn test_safe_filename() {
    for (expected_in, expected_out) in [
        (
            "allowed $special %characters",
            "allowed $special %characters",
        ),
        ("warp:drive", "warp_drive"),
        ("a/b/c/d:e", "a_b_c_d_e"),
        ("the\0sneaky\0null", "the_sneaky_null"),
        ("ascii\x03control\x1bchars", "ascii_control_chars"),
    ] {
        assert_eq!(safe_filename(expected_in), expected_out);
    }
}

