use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum MembershipRole {
    Owner,
    Admin,
    User,
}

impl MembershipRole {
    pub fn is_admin_or_owner(&self) -> bool {
        matches!(self, MembershipRole::Admin | MembershipRole::Owner)
    }

    pub fn is_owner(&self) -> bool {
        matches!(self, MembershipRole::Owner)
    }
}
