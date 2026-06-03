use warpui::AppContext;

use crate::search::command_search::searcher::CommandSearchItemAction;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::{DataSourceRunErrorWrapper, SyncDataSource};

/// Manages querying the AI queries in history for Command Search.
pub struct AIQueriesDataSource {}

impl AIQueriesDataSource {
    pub fn new() -> Self {
        Self {}
    }
}

impl SyncDataSource for AIQueriesDataSource {
    type Action = CommandSearchItemAction;

    /// Performs a query on the AI queries in history and returns a collection of matches.
    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let _ = (query, app);
        Ok(vec![])
    }
}
