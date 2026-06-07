use crate::cloud_object::{Owner, Space};
use crate::search::notebook_embedding::is_embed_accessible;
use crate::server::ids::ServerId;

#[test]
fn test_embed_in_personal_object() {
    assert!(is_embed_accessible(
        Space::Personal,
        Owner::mock_current_user()
    ));
    assert!(is_embed_accessible(
        Space::Personal,
        Owner::Team {
            team_uid: ServerId::from(123),
        }
    ));
}

