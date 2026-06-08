use warpui::AppContext;

use super::AcceptSlashCommandOrSavedPrompt;
use crate::search::async_snapshot_data_source::AsyncSnapshotDataSource;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::{BoxFuture, DataSourceRunErrorWrapper};

pub(crate) fn saved_prompts_data_source(
) -> AsyncSnapshotDataSource<(), AcceptSlashCommandOrSavedPrompt> {
    AsyncSnapshotDataSource::new(
        |_: &Query, _: &AppContext| (),
        fuzzy_match_saved_prompts,
    )
}

pub(crate) fn fuzzy_match_saved_prompts(
    _: (),
) -> BoxFuture<
    'static,
    Result<Vec<QueryResult<AcceptSlashCommandOrSavedPrompt>>, DataSourceRunErrorWrapper>,
> {
    Box::pin(async { Ok(vec![]) })
}
