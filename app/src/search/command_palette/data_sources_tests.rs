
use settings::manager::SettingsManager;
use warpui::{App, SingletonEntity};

use super::*;
use crate::auth::AuthStateProvider;
use crate::cloud_object::model::persistence::CloudModel;
use crate::cloud_object::model::view::CloudViewModel;
use crate::cloud_object::{CloudObject, CloudObjectMetadata, CloudObjectPermissions};
use crate::network::NetworkStatus;
use crate::search::data_source::Query;
use crate::server::cloud_objects::update_manager::UpdateManager;
use crate::server::ids::SyncId::{self};
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
use crate::server::server_api::ServerApiProvider;
use crate::settings::AISettings;
use crate::workflows::workflow::Workflow;
use crate::workflows::{CloudWorkflow, CloudWorkflowModel, WorkflowId};
use crate::workspaces::team_tester::TeamTesterStatus;
use crate::workspaces::user_profiles::UserProfiles;
use crate::workspaces::user_workspaces::UserWorkspaces;

fn mock_workflow(id: WorkflowId) -> CloudWorkflow {
    CloudWorkflow::new(
        SyncId::ServerId(id.into()),
        CloudWorkflowModel::new(Workflow::new(format!("foo{id}"), format!("bar{id}"))),
        CloudObjectMetadata::mock(),
        CloudObjectPermissions::mock_personal(),
    )
}

fn initialize_app(app: &mut App) {
    // Add the necessary singleton models to the App
    app.add_singleton_model(|_| NetworkStatus::new());
    app.add_singleton_model(|ctx| {
        UserWorkspaces::mock(vec![],
            ctx,
        )
    });
    app.add_singleton_model(TeamTesterStatus::new);
    app.add_singleton_model(CloudModel::mock);
    app.add_singleton_model(UpdateManager::mock);
    app.add_singleton_model(|_| UserProfiles::new(Vec::new()));
    app.add_singleton_model(CloudViewModel::new);
    app.add_singleton_model(|_| ServerApiProvider::new_for_test());
    app.add_singleton_model(|_| SettingsManager::default());
    app.add_singleton_model(|_| AuthStateProvider::new_for_test());
    app.update(crate::settings::init_and_register_user_preferences);
    app.update(AISettings::register_and_subscribe_to_events);
}

#[test]
fn test_drive_data_source_correctly_filters_drive_filter() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        // Initialize CloudModel
        CloudModel::handle(&app).update(&mut app, |model, ctx| {
            let wf = mock_workflow(2.into());
            model.create_object(wf.sync_id(), wf, ctx)
        });

        let mixer = app.add_model(|_| CommandPaletteMixer::new());
        let data_source_handle = app.add_model(warp_drive::DataSource::new);
        mixer.update(&mut app, |mixer, ctx| {
            // Add the drive data source with the relevant filters
            mixer.add_sync_source(
                data_source_handle,
                [
                    QueryFilter::Drive,
                    QueryFilter::Notebooks,
                    QueryFilter::Workflows,
                ],
            );

            // Run the query with the drive filter
            mixer.run_query(
                Query {
                    filters: HashSet::from([QueryFilter::Drive]),
                    text: "foo".into(),
                },
                ctx,
            );
        });

        app.read(|app| {
            let results = mixer.as_ref(app).results();

            // Expect both of the results to be included
            assert_eq!(results.len(), 1);
        });
    })
}

#[test]
fn test_drive_data_source_correctly_filters_no_filter() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        // Initialize CloudModel
        CloudModel::handle(&app).update(&mut app, |model, ctx| {
            let wf = mock_workflow(2.into());
            model.create_object(wf.sync_id(), wf, ctx)
        });
        let mixer = app.add_model(|_| CommandPaletteMixer::new());
        let data_source_handle = app.add_model(warp_drive::DataSource::new);
        mixer.update(&mut app, |mixer, ctx| {
            // Add the drive data source with the relevant filters
            mixer.add_sync_source(
                data_source_handle,
                [
                    QueryFilter::Drive,
                    QueryFilter::Notebooks,
                    QueryFilter::Workflows,
                ],
            );

            // Run the query with no filter
            mixer.run_query(
                Query {
                    filters: HashSet::new(),
                    text: "foo".into(),
                },
                ctx,
            );
        });

        app.read(|app| {
            let results = mixer.as_ref(app).results();

            // Expect both of the results to be included
            assert_eq!(results.len(), 1);
        });
    })
}

#[test]
fn test_drive_data_source_correctly_filters_workflow_filter() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        // Initialize CloudModel
        CloudModel::handle(&app).update(&mut app, |model, ctx| {
            let wf = mock_workflow(2.into());
            model.create_object(wf.sync_id(), wf, ctx)
        });
        let mixer = app.add_model(|_| CommandPaletteMixer::new());
        let data_source_handle = app.add_model(warp_drive::DataSource::new);
        mixer.update(&mut app, |mixer, ctx| {
            // Add the drive data source with the relevant filters
            mixer.add_sync_source(
                data_source_handle,
                [
                    QueryFilter::Drive,
                    QueryFilter::Notebooks,
                    QueryFilter::Workflows,
                ],
            );

            // Run the query with no filter
            mixer.run_query(
                Query {
                    filters: HashSet::from([QueryFilter::Workflows]),
                    text: "foo".into(),
                },
                ctx,
            );
        });

        app.read(|app| {
            let results = mixer.as_ref(app).results();

            // Expect only the workflow result to be included
            assert_eq!(results.len(), 1);

            assert!(results[0].accessibility_label().starts_with("Workflow:"));
        });
    })
}
