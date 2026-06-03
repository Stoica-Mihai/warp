use std::path::PathBuf;

use warpui::{AppContext, SingletonEntity};

use crate::ai::agent::{SearchCodebaseFailureReason, SearchCodebaseResult};
use crate::ai::blocklist::SessionContext;
use crate::remote_server::codebase_index_model::RemoteCodebaseIndexModel;

pub(super) enum RemoteSearchRequest {
    Pending(futures_util::stream::AbortHandle),
    Ready(SearchCodebaseResult),
}

pub(super) fn root_directory_for_search(
    session_context: &SessionContext,
    requested_codebase_path: Option<&str>,
    app: &AppContext,
) -> Option<PathBuf> {
    RemoteCodebaseIndexModel::as_ref(app)
        .active_repo_path(session_context, requested_codebase_path)
        .or_else(|| {
            requested_codebase_path
                .is_none()
                .then(|| session_context.current_working_directory().clone())
                .flatten()
        })
        .map(PathBuf::from)
}

pub(super) fn send_request() -> RemoteSearchRequest {
    RemoteSearchRequest::Ready(SearchCodebaseResult::Failed {
        reason: SearchCodebaseFailureReason::CodebaseNotIndexed,
        message: "Remote codebase search is not enabled.".to_string(),
    })
}
