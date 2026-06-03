//! Snapshot upload pipeline for local-to-cloud handoff.

use remote_server::proto::UploadHandoffSnapshotResponse;

use crate::server::server_api::ai::InitialSnapshotToken;

/// Convert a `Result<Option<InitialSnapshotToken>>` into an `UploadHandoffSnapshotResponse` proto.
///
/// Used by `server_model.rs::handle_upload_handoff_snapshot` to build the response.
pub(crate) fn upload_result_to_proto(
    result: Result<Option<InitialSnapshotToken>, anyhow::Error>,
) -> UploadHandoffSnapshotResponse {
    match result {
        Ok(Some(token)) => UploadHandoffSnapshotResponse {
            initial_snapshot_token: Some(token.as_str().to_string()),
            success: true,
            error: None,
        },
        Ok(None) => UploadHandoffSnapshotResponse {
            initial_snapshot_token: None,
            success: true,
            error: None,
        },
        Err(e) => UploadHandoffSnapshotResponse {
            initial_snapshot_token: None,
            success: false,
            error: Some(format!("{e:#}")),
        },
    }
}
