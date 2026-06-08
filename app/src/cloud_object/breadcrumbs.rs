use warpui::AppContext;

use super::Space;
use crate::drive::WarpDriveItemId;
use crate::drive::CloudObjectTypeAndId;

// Encapsulates an object that can contain other objects, and keeps
// information necessary to show breadcrumbs.
#[derive(Clone, Debug)]
pub struct ContainingObject {
    pub name: String,
    pub kind: ContainingObjectKind,
}

impl Space {
    pub fn into_containing_object(self, app: &AppContext) -> ContainingObject {
        ContainingObject {
            name: self.name(app).clone(),
            kind: ContainingObjectKind::Space(self),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ContainingObjectKind {
    Space(Space),
    Object(CloudObjectTypeAndId),
}

impl ContainingObjectKind {
    pub fn into_item_id(self) -> WarpDriveItemId {
        match self {
            ContainingObjectKind::Space(space) => WarpDriveItemId::Space(space),
            ContainingObjectKind::Object(object) => WarpDriveItemId::Object(object),
        }
    }
}
