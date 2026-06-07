use super::{
    CloudObjectEventEntrypoint, GenericStringObjectFormat, GenericStringObjectUniqueKey, Owner,
    SerializedModel,
};
use crate::ids::{ClientId, FolderId};

/// Helper struct that contains all the info needed to create an object on the server.
pub struct CreateObjectRequest {
    pub serialized_model: Option<SerializedModel>,
    pub title: Option<String>,
    pub owner: Owner,
    pub client_id: ClientId,
    pub initial_folder_id: Option<FolderId>,
    pub entrypoint: CloudObjectEventEntrypoint,
}

#[derive(PartialEq, Eq, Debug)]
pub struct BulkCreateGenericStringObjectsRequest {
    pub id: ClientId,
    pub format: GenericStringObjectFormat,
    pub uniqueness_key: Option<GenericStringObjectUniqueKey>,
    pub serialized_model: SerializedModel,
    pub initial_folder_id: Option<FolderId>,
    pub entrypoint: CloudObjectEventEntrypoint,
}

