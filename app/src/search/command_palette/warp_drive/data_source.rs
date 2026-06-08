use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use crate::cloud_object::model::persistence::{CloudModel, CloudModelEvent};
use crate::cloud_object::{CloudObject, CloudObjectLocation, ObjectType};
use crate::search::command_palette::mixer::CommandPaletteItemAction;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::server::ids::{ObjectUid, SyncId};

/// Datasource that searches against all Warp Drive objects
pub struct DataSource {
    searcher: FuzzyWarpDriveSearcher,
}

impl DataSource {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        ctx.subscribe_to_model(&CloudModel::handle(ctx), Self::handle_cloud_object_updated);
        let mut searcher = FuzzyWarpDriveSearcher;
        searcher.refresh_search_index(ctx);
        DataSource { searcher }
    }

    fn handle_cloud_object_updated(
        &mut self,
        event: &CloudModelEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        if let CloudModelEvent::InitialLoadCompleted = event {
            self.searcher.refresh_search_index(ctx);
            return;
        }

        match event {
            CloudModelEvent::ObjectCreated { type_and_id }
            | CloudModelEvent::ObjectUntrashed { type_and_id, .. }
            | CloudModelEvent::ObjectMoved { type_and_id, .. }
            | CloudModelEvent::ObjectUpdated { type_and_id, .. } => {
                if let Some(obj) = CloudModel::as_ref(ctx).get_by_uid(&type_and_id.uid()) {
                    self.searcher.insert_searchable_object(obj, type_and_id.object_type(), ctx);
                } else {
                    log::error!("Object with ID {type_and_id:?} not found in CloudModel");
                }
            }
            CloudModelEvent::ObjectTrashed { type_and_id, .. } => {
                self.searcher.delete_searchable_object(type_and_id.uid(), type_and_id.object_type(), ctx);
            }
            CloudModelEvent::ObjectSynced { type_and_id, client_id, server_id } => {
                let Some(cloud_object) = CloudModel::as_ref(ctx).get_by_uid(&server_id.uid())
                else {
                    return;
                };
                self.searcher.delete_searchable_object(client_id.to_string(), type_and_id.object_type(), ctx);
                self.searcher.insert_searchable_object(cloud_object, type_and_id.object_type(), ctx);
            }
            _ => {}
        }
    }
}

impl crate::search::mixer::SyncDataSource for DataSource {
    type Action = CommandPaletteItemAction;

    fn run_query(
        &self,
        _query: &Query,
        _app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        Ok(vec![])
    }
}

impl DataSource {
    pub fn query_result(
        &self,
        _sync_id: &SyncId,
        _app: &AppContext,
    ) -> Option<QueryResult<CommandPaletteItemAction>> {
        None
    }
}

impl Entity for DataSource {
    type Event = ();
}

struct FuzzyWarpDriveSearcher;

impl FuzzyWarpDriveSearcher {
    fn insert_searchable_object(
        &mut self,
        object: &dyn CloudObject,
        object_type: ObjectType,
        app: &AppContext,
    ) {
        let _ = (object, object_type, app);
    }

    fn delete_searchable_object(
        &mut self,
        uid: ObjectUid,
        object_type: ObjectType,
        app: &AppContext,
    ) {
        let _ = (uid, object_type, app);
    }

    fn refresh_search_index(&mut self, _app: &AppContext) {}
}
