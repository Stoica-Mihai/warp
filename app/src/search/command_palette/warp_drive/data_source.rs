use std::collections::HashMap;

use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use super::notebook_search_item::NotebookSearchItem;
use super::workflow_search_item::WorkflowSearchItem;
use crate::cloud_object::model::persistence::{CloudModel, CloudModelEvent};
use crate::cloud_object::{
    CloudObject, CloudObjectLocation, ObjectType,
};
use crate::drive::folders::CloudFolder;
use crate::notebooks::CloudNotebook;
use crate::search::command_palette::mixer::CommandPaletteItemAction;
use crate::search::data_source::{DataSourceSearchError, Query, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::search::notebooks::fuzzy_match::FuzzyMatchNotebookResult;
use crate::search::workflows::fuzzy_match::FuzzyMatchWorkflowResult;
use crate::search::QueryFilter;
use crate::server::ids::{ObjectUid, SyncId};
use crate::settings::AISettings;
use crate::workflows::CloudWorkflow;

/// Datasource that searches against all Warp Drive objects
pub struct DataSource {
    searcher: Box<dyn WarpDriveSearcher>,
}

impl DataSource {
    #[cfg(not(target_family = "wasm"))]
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        if warp_core::features::FeatureFlag::UseTantivySearch.is_enabled() {
            Self::new_full_text(ctx)
        } else {
            Self::new_fuzzy(ctx)
        }
    }

    #[cfg(target_family = "wasm")]
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_fuzzy(ctx)
    }

    pub fn new_fuzzy(ctx: &mut ModelContext<Self>) -> Self {
        ctx.subscribe_to_model(&CloudModel::handle(ctx), Self::handle_cloud_object_updated);
        let mut searcher = Box::new(FuzzyWarpDriveSearcher::default());
        searcher.refresh_search_index(ctx).unwrap_or_else(|err| {
            log::error!("Error refreshing search index: {err:?}");
        });
        DataSource { searcher }
    }

    #[cfg(not(target_family = "wasm"))]
    fn new_full_text(ctx: &mut ModelContext<Self>) -> Self {
        ctx.subscribe_to_model(&CloudModel::handle(ctx), Self::handle_cloud_object_updated);
        let mut searcher = Box::new(full_text_searcher::FullTextWarpDriveSearcher::new(
            ctx.background_executor(),
        ));
        searcher.refresh_search_index(ctx).unwrap_or_else(|err| {
            log::error!("Error refreshing search index: {err:?}");
        });
        DataSource { searcher }
    }

    fn handle_cloud_object_updated(
        &mut self,
        event: &CloudModelEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        if let CloudModelEvent::InitialLoadCompleted = event {
            self.searcher
                .refresh_search_index(ctx)
                .unwrap_or_else(|err| {
                    log::error!("Error refreshing search index after initial load: {err:?}");
                });
            return;
        }

        match event {
            CloudModelEvent::ObjectCreated { type_and_id }
            | CloudModelEvent::ObjectUntrashed { type_and_id, .. }
            | CloudModelEvent::ObjectMoved { type_and_id, .. }
            | CloudModelEvent::ObjectUpdated { type_and_id, .. } => {
                if let Some(obj) = CloudModel::as_ref(ctx).get_by_uid(&type_and_id.uid()) {
                    self.searcher
                        .insert_searchable_object(obj, type_and_id.object_type(), ctx)
                        .unwrap_or_else(|err| {
                            log::error!("Error inserting object into search index: {err:?}");
                        });
                } else {
                    log::error!("Object with ID {type_and_id:?} not found in CloudModel");
                }
            }
            CloudModelEvent::ObjectTrashed { type_and_id, .. } => self
                .searcher
                .delete_searchable_object(type_and_id.uid(), type_and_id.object_type(), ctx)
                .unwrap_or_else(|err| {
                    log::error!("Error deleting object from search index: {err:?}");
                }),
            CloudModelEvent::ObjectSynced {
                type_and_id,
                client_id,
                server_id,
            } => {
                let Some(cloud_object) = CloudModel::as_ref(ctx).get_by_uid(&server_id.uid())
                else {
                    return;
                };

                self.searcher
                    .delete_searchable_object(client_id.to_string(), type_and_id.object_type(), ctx)
                    .unwrap_or_else(|err| {
                        log::warn!("Error deleting object from search index: {err:?}");
                    });

                self.searcher
                    .insert_searchable_object(cloud_object, type_and_id.object_type(), ctx)
                    .unwrap_or_else(|err| {
                        log::warn!("Error inserting object into search index: {err:?}");
                    });
            }
            _ => {}
        }
    }

    pub fn search_workflows(
        &self,
        query: &Query,
        should_include_agent_mode_prompts: bool,
        should_include_command_workflows: bool,
        app: &AppContext,
    ) -> anyhow::Result<Vec<WorkflowSearchItem>> {
        self.searcher.search_workflow(
            &query.text.to_lowercase(),
            app,
            should_include_agent_mode_prompts,
            should_include_command_workflows,
        )
    }
}

impl crate::search::mixer::SyncDataSource for DataSource {
    type Action = CommandPaletteItemAction;

    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let mut filtered_cloud_objects = Vec::new();

        let should_include_all_drive_objects = Self::include_all_drive_objects_in_result(query);

        if query.filters.contains(&QueryFilter::Notebooks) || should_include_all_drive_objects {
            filtered_cloud_objects.extend(
                self.searcher
                    .search_notebook(&query.text.to_lowercase(), app)
                    .map_err(|err| {
                        Box::new(DataSourceSearchError {
                            message: err.to_string(),
                        }) as DataSourceRunErrorWrapper
                    })?
                    .into_iter()
                    .map(QueryResult::from),
            );
        }

        if query.filters.contains(&QueryFilter::Plans) || should_include_all_drive_objects {
            filtered_cloud_objects.extend(
                self.searcher
                    .search_plans(&query.text.to_lowercase(), app)
                    .map_err(|err| {
                        Box::new(DataSourceSearchError {
                            message: err.to_string(),
                        }) as DataSourceRunErrorWrapper
                    })?
                    .into_iter()
                    .map(QueryResult::from),
            );
        }

        let should_include_agent_mode_prompts =
            (query.filters.contains(&QueryFilter::AgentModeWorkflows)
                || should_include_all_drive_objects)
                && AISettings::as_ref(app).is_any_ai_enabled(app);
        let should_include_command_workflows =
            query.filters.contains(&QueryFilter::Workflows) || should_include_all_drive_objects;

        if should_include_agent_mode_prompts || should_include_command_workflows {
            filtered_cloud_objects.extend(
                self.search_workflows(
                    query,
                    should_include_agent_mode_prompts,
                    should_include_command_workflows,
                    app,
                )
                .map_err(|err| {
                    Box::new(DataSourceSearchError {
                        message: err.to_string(),
                    }) as DataSourceRunErrorWrapper
                })?
                .into_iter()
                .map(QueryResult::from),
            );
        }

        Ok(filtered_cloud_objects)
    }
}

impl DataSource {
    fn include_all_drive_objects_in_result(query: &Query) -> bool {
        query.filters.contains(&QueryFilter::Drive) || query.filters.is_empty()
    }

    pub fn query_result(
        &self,
        sync_id: &SyncId,
        app: &AppContext,
    ) -> Option<QueryResult<CommandPaletteItemAction>> {
        let object = CloudModel::as_ref(app).get_by_uid(&sync_id.uid())?;
        let workflow: Option<&CloudWorkflow> = object.into();
        if let Some(workflow) = workflow {
            return Some(QueryResult::from(WorkflowSearchItem {
                match_result: FuzzyMatchWorkflowResult::no_match(),
                cloud_workflow: workflow.clone(),
            }));
        }

        let notebook: Option<&CloudNotebook> = object.into();
        if let Some(notebook) = notebook {
            return Some(QueryResult::from(NotebookSearchItem {
                match_result: FuzzyMatchNotebookResult::no_match(),
                cloud_notebook: notebook.clone(),
            }));
        }

        None
    }
}

impl Entity for DataSource {
    type Event = ();
}

trait WarpDriveSearcher {
    fn insert_searchable_object(
        &mut self,
        object: &dyn CloudObject,
        object_type: ObjectType,
        app: &AppContext,
    ) -> anyhow::Result<()>;

    fn delete_searchable_object(
        &mut self,
        uid: ObjectUid,
        object_type: ObjectType,
        app: &AppContext,
    ) -> anyhow::Result<()>;

    fn refresh_search_index(&mut self, app: &AppContext) -> anyhow::Result<()>;

    fn search_notebook(
        &self,
        query: &str,
        app: &AppContext,
    ) -> anyhow::Result<Vec<NotebookSearchItem>>;

    fn search_workflow(
        &self,
        query: &str,
        app: &AppContext,
        should_include_am_prompts: bool,
        should_include_command_workflow: bool,
    ) -> anyhow::Result<Vec<WorkflowSearchItem>>;

    fn search_plans(
        &self,
        query: &str,
        app: &AppContext,
    ) -> anyhow::Result<Vec<NotebookSearchItem>>;
}

#[derive(Default)]
struct FuzzyWarpDriveSearcher {
    notebooks: HashMap<ObjectUid, CloudNotebook>,
    workflows: HashMap<ObjectUid, CloudWorkflow>,
}

impl WarpDriveSearcher for FuzzyWarpDriveSearcher {
    fn insert_searchable_object(
        &mut self,
        object: &dyn CloudObject,
        object_type: ObjectType,
        app: &AppContext,
    ) -> anyhow::Result<()> {
        match object_type {
            ObjectType::Notebook => {
                let notebook: Option<&CloudNotebook> = object.into();
                if let Some(notebook) = notebook {
                    self.notebooks.insert(notebook.uid(), notebook.clone());
                } else {
                    anyhow::bail!("Expected CloudNotebook, got {:?}", object);
                }
            }
            ObjectType::Workflow => {
                let workflow: Option<&CloudWorkflow> = object.into();
                if let Some(workflow) = workflow {
                    self.workflows.insert(workflow.uid(), workflow.clone());
                } else {
                    anyhow::bail!("Expected CloudWorkflow, got {:?}", object);
                }
            }
            ObjectType::Folder => {
                let folder: Option<&CloudFolder> = object.into();
                if let Some(folder) = folder {
                    let location = CloudObjectLocation::Folder(folder.id);
                    for obj in CloudModel::as_ref(app)
                        .active_cloud_objects_in_location_without_descendents(location, app)
                    {
                        self.insert_searchable_object(obj, obj.object_type(), app)?
                    }
                } else {
                    anyhow::bail!("Expected CloudFolder, got {:?}", object);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn delete_searchable_object(
        &mut self,
        uid: ObjectUid,
        object_type: ObjectType,
        app: &AppContext,
    ) -> anyhow::Result<()> {
        match object_type {
            ObjectType::Notebook => {
                self.notebooks.remove(&uid);
            }
            ObjectType::Workflow => {
                self.workflows.remove(&uid);
            }
            ObjectType::Folder => {
                let model = CloudModel::as_ref(app);
                let Some(obj) = model.get_by_uid(&uid) else {
                    anyhow::bail!("Object with ID {:?} not found in CloudModel", uid);
                };
                let folder: Option<&CloudFolder> = obj.into();
                if let Some(folder) = folder {
                    let location = CloudObjectLocation::Folder(folder.id);
                    for obj in
                        model.trashed_cloud_objects_in_location_without_descendents(location, app)
                    {
                        self.delete_searchable_object(obj.uid(), obj.object_type(), app)?
                    }
                } else {
                    anyhow::bail!("Expected CloudFolder, got {:?}", obj);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn refresh_search_index(&mut self, app: &AppContext) -> anyhow::Result<()> {
        self.workflows.clear();
        self.notebooks.clear();
        let model = CloudModel::as_ref(app);
        let active_uids = model.active_object_uids();
        for object in model.cloud_objects() {
            if !active_uids.contains(&object.uid()) {
                continue;
            }
            if let Some(workflow) = <Option<&CloudWorkflow>>::from(object.as_ref()) {
                self.workflows.insert(workflow.uid(), workflow.clone());
            } else if let Some(notebook) = <Option<&CloudNotebook>>::from(object.as_ref()) {
                self.notebooks.insert(notebook.uid(), notebook.clone());
            }
        }
        Ok(())
    }

    fn search_notebook(
        &self,
        query: &str,
        app: &AppContext,
    ) -> anyhow::Result<Vec<NotebookSearchItem>> {
        let cloud_notebooks = CloudModel::as_ref(app).get_all_active_notebooks();
        Ok(cloud_notebooks
            .filter_map(|cloud_notebook| {
                FuzzyMatchNotebookResult::try_match(query, cloud_notebook, app).map(
                    |match_result| NotebookSearchItem {
                        match_result,
                        cloud_notebook: cloud_notebook.clone(),
                    },
                )
            })
            .collect())
    }

    fn search_plans(
        &self,
        query: &str,
        app: &AppContext,
    ) -> anyhow::Result<Vec<NotebookSearchItem>> {
        let cloud_notebooks = CloudModel::as_ref(app)
            .get_all_active_notebooks()
            .filter(|notebook| notebook.model().ai_document_id.is_some());
        Ok(cloud_notebooks
            .filter_map(|cloud_notebook| {
                FuzzyMatchNotebookResult::try_match(query, cloud_notebook, app).map(
                    |match_result| NotebookSearchItem {
                        match_result,
                        cloud_notebook: cloud_notebook.clone(),
                    },
                )
            })
            .collect())
    }

    fn search_workflow(
        &self,
        query: &str,
        app: &AppContext,
        should_include_am_prompts: bool,
        should_include_command_workflow: bool,
    ) -> anyhow::Result<Vec<WorkflowSearchItem>> {
        let cloud_workflows = CloudModel::as_ref(app).get_all_active_workflows();

        Ok(cloud_workflows
            .filter_map(move |cloud_workflow| {
                if !should_include_am_prompts
                    && cloud_workflow.model().data.is_agent_mode_workflow()
                    || !should_include_command_workflow
                        && cloud_workflow.model().data.is_command_workflow()
                {
                    return None;
                };
                FuzzyMatchWorkflowResult::try_match(
                    query,
                    &cloud_workflow.model().data,
                    cloud_workflow.breadcrumbs(app).as_str(),
                )
                .map(|match_result| WorkflowSearchItem {
                    match_result,
                    cloud_workflow: cloud_workflow.clone(),
                })
            })
            .collect())
    }
}

#[cfg(not(target_family = "wasm"))]
mod full_text_searcher {
    use std::sync::Arc;

    use fuzzy_match::FuzzyMatchResult;
    use warpui::r#async::executor::Background;
    use warpui::{AppContext, SingletonEntity};

    use crate::cloud_object::model::persistence::CloudModel;
    use crate::cloud_object::{
        CloudObject, CloudObjectLocation, ObjectType,
    };
    use crate::define_search_schema;
    use crate::drive::folders::CloudFolder;
    use crate::notebooks::manager::NotebookManager;
    use crate::notebooks::CloudNotebook;
    use crate::search::command_palette::warp_drive::data_source::WarpDriveSearcher;
    use crate::search::command_palette::warp_drive::notebook_search_item::NotebookSearchItem;
    use crate::search::command_palette::warp_drive::workflow_search_item::WorkflowSearchItem;
    use crate::search::notebooks::fuzzy_match::FuzzyMatchNotebookResult;
    use crate::search::searcher::{AsyncSearcher, DEFAULT_MEMORY_BUDGET, SCORE_CONVERSION_FACTOR};
    use crate::search::workflows::fuzzy_match::FuzzyMatchWorkflowResult;
    use crate::server::ids::ObjectUid;
    use crate::workflows::CloudWorkflow;

    const MEMORY_BUDGET: usize = 100_000_000;

    define_search_schema!(
        schema_name: NOTEBOOK_SEARCH_SCHEMA,
        config_name: NotebookConfig,
        search_doc: NotebookSearchDocument,
        identifying_doc: NotebookIdDocument,
        search_fields: [
            name: 0.6,
            content: 0.2,
            folder: 0.2
        ],
        id_fields: [
            uid: String
        ],
        boost_factor: 1.15
    );
    define_search_schema!(
        schema_name: WORKFLOW_SEARCH_SCHEMA,
        config_name: WorkflowConfig,
        search_doc: WorkflowSearchDocument,
        identifying_doc: WorkflowIdDocument,
        search_fields: [
            name: 0.5,
            content: 0.3,
            description: 0.1,
            folder: 0.1
        ],
        id_fields: [
            uid: String
        ],
        boost_factor: 1.3
    );

    pub(crate) struct FullTextWarpDriveSearcher {
        notebook_searcher: AsyncSearcher<NotebookConfig>,
        workflow_searcher: AsyncSearcher<WorkflowConfig>,
    }

    impl FullTextWarpDriveSearcher {
        fn search_notebooks_with_filter(
            &self,
            query: &str,
            filter_by_plan: bool,
            app: &AppContext,
        ) -> anyhow::Result<Vec<NotebookSearchItem>> {
            if query.is_empty() {
                return Ok(self
                    .notebook_searcher
                    .get_all_doc_ids()?
                    .into_iter()
                    .filter_map(|search_match| {
                        let notebook: Option<&CloudNotebook> = CloudModel::as_ref(app)
                            .get_by_uid(&search_match.uid)?
                            .into();
                        let cloud_notebook = notebook?;
                        if filter_by_plan && cloud_notebook.model().ai_document_id.is_none() {
                            return None;
                        }

                        Some(NotebookSearchItem {
                            match_result: FuzzyMatchNotebookResult::no_match(),
                            cloud_notebook: cloud_notebook.clone(),
                        })
                    })
                    .collect());
            }

            Ok(self
                .notebook_searcher
                .search_id(query)?
                .into_iter()
                .filter_map(|search_match| {
                    let notebook: Option<&CloudNotebook> = CloudModel::as_ref(app)
                        .get_by_uid(&search_match.values.uid)?
                        .into();
                    let notebook = notebook?;

                    if filter_by_plan && notebook.model().ai_document_id.is_none() {
                        return None;
                    }

                    let name_match_result = Some(FuzzyMatchResult {
                        score: (search_match.score * SCORE_CONVERSION_FACTOR) as i64,
                        matched_indices: search_match.highlights.name,
                    });
                    let content_match_result = Some(FuzzyMatchResult {
                        score: (search_match.score * SCORE_CONVERSION_FACTOR) as i64,
                        matched_indices: search_match.highlights.content,
                    });
                    let folder_match_result = Some(FuzzyMatchResult {
                        score: (search_match.score * SCORE_CONVERSION_FACTOR) as i64,
                        matched_indices: search_match.highlights.folder,
                    });

                    Some(NotebookSearchItem {
                        match_result: FuzzyMatchNotebookResult {
                            name_match_result,
                            content_match_result,
                            folder_match_result,
                        },
                        cloud_notebook: notebook.clone(),
                    })
                })
                .collect())
        }
    }

    impl WarpDriveSearcher for FullTextWarpDriveSearcher {
        fn insert_searchable_object(
            &mut self,
            object: &dyn CloudObject,
            object_type: ObjectType,
            app: &AppContext,
        ) -> anyhow::Result<()> {
            match object_type {
                ObjectType::Notebook => {
                    let notebook: Option<&CloudNotebook> = object.into();
                    if let Some(notebook) = notebook {
                        let name = notebook.model().title.to_lowercase();
                        let content = NotebookManager::as_ref(app)
                            .notebook_raw_text(notebook.id)
                            .unwrap_or(&notebook.model().data)
                            .to_lowercase();
                        let folder = notebook.breadcrumbs(app).to_lowercase();
                        let uid = notebook.uid();

                        let document = NotebookSearchDocument {
                            name,
                            content,
                            folder,
                            uid,
                        };
                        self.notebook_searcher.insert_document_async(document)
                    } else {
                        anyhow::bail!("Expected CloudNotebook, got {:?}", object);
                    }
                }
                ObjectType::Workflow => {
                    let workflow: Option<&CloudWorkflow> = object.into();
                    if let Some(cloud_workflow) = workflow {
                        let workflow = &cloud_workflow.model().data;

                        let title = workflow.name().to_lowercase();
                        let content = workflow.content().to_lowercase();
                        let description = workflow
                            .description()
                            .unwrap_or(&"".to_owned())
                            .to_lowercase();
                        let folder = cloud_workflow.breadcrumbs(app).to_lowercase();

                        let document = WorkflowSearchDocument {
                            name: title,
                            content,
                            description,
                            folder,
                            uid: cloud_workflow.uid(),
                        };
                        self.workflow_searcher.insert_document_async(document)
                    } else {
                        anyhow::bail!("Expected CloudWorkflow, got {:?}", object);
                    }
                }
                ObjectType::Folder => {
                    let folder: Option<&CloudFolder> = object.into();
                    if let Some(folder) = folder {
                        let location = CloudObjectLocation::Folder(folder.id);
                        for obj in CloudModel::as_ref(app)
                            .active_cloud_objects_in_location_without_descendents(location, app)
                        {
                            self.insert_searchable_object(obj, obj.object_type(), app)?
                        }
                        Ok(())
                    } else {
                        anyhow::bail!("Expected CloudFolder, got {:?}", object);
                    }
                }
                _ => Ok(()),
            }
        }

        fn delete_searchable_object(
            &mut self,
            uid: ObjectUid,
            object_type: ObjectType,
            app: &AppContext,
        ) -> anyhow::Result<()> {
            match object_type {
                ObjectType::Notebook => {
                    let identifying_entry = NotebookIdDocument { uid };
                    self.notebook_searcher
                        .delete_document_async(identifying_entry)
                }
                ObjectType::Workflow => {
                    let identifying_entry = WorkflowIdDocument { uid };
                    self.workflow_searcher
                        .delete_document_async(identifying_entry)
                }
                ObjectType::Folder => {
                    let Some(obj) = CloudModel::as_ref(app).get_by_uid(&uid) else {
                        anyhow::bail!("Object with ID {:?} not found in CloudModel", uid);
                    };
                    let folder: Option<&CloudFolder> = obj.into();
                    if let Some(folder) = folder {
                        let location = CloudObjectLocation::Folder(folder.id);
                        for obj in CloudModel::as_ref(app)
                            .trashed_cloud_objects_in_location_without_descendents(location, app)
                        {
                            self.delete_searchable_object(obj.uid(), obj.object_type(), app)?
                        }
                        Ok(())
                    } else {
                        anyhow::bail!("Expected CloudFolder, got {:?}", folder);
                    }
                }
                _ => Ok(()),
            }
        }

        fn refresh_search_index(&mut self, app: &AppContext) -> anyhow::Result<()> {
            let model = CloudModel::as_ref(app);
            let active_uids = model.active_object_uids();

            self.notebook_searcher.clear_search_index_async()?;
            let notebook_docs = model
                .cloud_objects()
                .filter(|obj| active_uids.contains(&obj.uid()))
                .filter_map(|obj| {
                    let notebook: Option<&CloudNotebook> = obj.as_ref().into();
                    notebook.map(|notebook| {
                        let name = notebook.model().title.to_lowercase();
                        let content = NotebookManager::as_ref(app)
                            .notebook_raw_text(notebook.id)
                            .unwrap_or(&notebook.model().data)
                            .to_lowercase();
                        let folder = notebook.breadcrumbs(app).to_lowercase();
                        let uid = notebook.uid();
                        NotebookSearchDocument {
                            name,
                            content,
                            folder,
                            uid,
                        }
                    })
                });
            self.notebook_searcher.build_index_async(notebook_docs)?;

            self.workflow_searcher.clear_search_index_async()?;
            let workflow_docs = model
                .cloud_objects()
                .filter(|obj| active_uids.contains(&obj.uid()))
                .filter_map(|obj| {
                    let cloud_workflow: Option<&CloudWorkflow> = obj.as_ref().into();
                    cloud_workflow.map(|cloud_workflow| {
                        let workflow = &cloud_workflow.model().data;
                        let title = workflow.name().to_lowercase();
                        let content = workflow.content().to_lowercase();
                        let description = workflow
                            .description()
                            .unwrap_or(&"".to_owned())
                            .to_lowercase();
                        let folder = cloud_workflow.breadcrumbs(app).to_lowercase();
                        WorkflowSearchDocument {
                            name: title,
                            content,
                            description,
                            folder,
                            uid: cloud_workflow.uid(),
                        }
                    })
                });
            self.workflow_searcher.build_index_async(workflow_docs)?;

            Ok(())
        }

        fn search_notebook(
            &self,
            query: &str,
            app: &AppContext,
        ) -> anyhow::Result<Vec<NotebookSearchItem>> {
            self.search_notebooks_with_filter(query, false, app)
        }

        fn search_plans(
            &self,
            query: &str,
            app: &AppContext,
        ) -> anyhow::Result<Vec<NotebookSearchItem>> {
            self.search_notebooks_with_filter(query, true, app)
        }

        fn search_workflow(
            &self,
            query: &str,
            app: &AppContext,
            should_include_am_prompts: bool,
            should_include_command_workflow: bool,
        ) -> anyhow::Result<Vec<WorkflowSearchItem>> {
            if query.is_empty() {
                return Ok(self
                    .workflow_searcher
                    .get_all_doc_ids()?
                    .into_iter()
                    .filter_map(|search_match| {
                        let cloud_workflow: Option<&CloudWorkflow> = CloudModel::as_ref(app)
                            .get_by_uid(&search_match.uid)?
                            .into();
                        let cloud_workflow = cloud_workflow?;
                        let workflow = &cloud_workflow.model().data;

                        if !should_include_am_prompts && workflow.is_agent_mode_workflow()
                            || !should_include_command_workflow && workflow.is_command_workflow()
                        {
                            return None;
                        }

                        Some(WorkflowSearchItem {
                            match_result: FuzzyMatchWorkflowResult::no_match(),
                            cloud_workflow: cloud_workflow.clone(),
                        })
                    })
                    .collect());
            }

            Ok(self
                .workflow_searcher
                .search_id(query)?
                .into_iter()
                .filter_map(|search_match| {
                    let cloud_workflow: Option<&CloudWorkflow> = CloudModel::as_ref(app)
                        .get_by_uid(&search_match.values.uid)?
                        .into();
                    let cloud_workflow = cloud_workflow?;
                    let workflow = &cloud_workflow.model().data;

                    if !should_include_am_prompts && workflow.is_agent_mode_workflow()
                        || !should_include_command_workflow && workflow.is_command_workflow()
                    {
                        return None;
                    }

                    let name_match_result = Some(FuzzyMatchResult {
                        score: (search_match.score * SCORE_CONVERSION_FACTOR) as i64,
                        matched_indices: search_match.highlights.name,
                    });
                    let content_match_result = Some(FuzzyMatchResult {
                        score: (search_match.score * SCORE_CONVERSION_FACTOR) as i64,
                        matched_indices: search_match.highlights.content,
                    });
                    let description_match_result = Some(FuzzyMatchResult {
                        score: (search_match.score * SCORE_CONVERSION_FACTOR) as i64,
                        matched_indices: search_match.highlights.description,
                    });
                    let folder_match_result = Some(FuzzyMatchResult {
                        score: (search_match.score * SCORE_CONVERSION_FACTOR) as i64,
                        matched_indices: search_match.highlights.folder,
                    });

                    Some(WorkflowSearchItem {
                        match_result: FuzzyMatchWorkflowResult {
                            name_match_result,
                            content_match_result,
                            description_match_result,
                            folder_match_result,
                        },
                        cloud_workflow: cloud_workflow.clone(),
                    })
                })
                .collect())
        }
    }

    impl FullTextWarpDriveSearcher {
        pub(crate) fn new(background: Arc<Background>) -> Self {
            FullTextWarpDriveSearcher {
                notebook_searcher: NOTEBOOK_SEARCH_SCHEMA
                    .create_async_searcher(MEMORY_BUDGET, background.clone()),
                workflow_searcher: WORKFLOW_SEARCH_SCHEMA
                    .create_async_searcher(DEFAULT_MEMORY_BUDGET, background),
            }
        }
    }
}
