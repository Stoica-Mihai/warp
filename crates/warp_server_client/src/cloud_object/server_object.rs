/// Tracks whether a cloud object has a conflict with a server version.
///
/// Server sync is removed in this local-only build, so conflicts never occur; this is retained
/// as runtime state that is always `NoConflicts`.
#[derive(Clone, Debug, Default)]
pub enum ConflictStatus {
    #[default]
    NoConflicts,
}

impl ConflictStatus {
    pub fn has_conflicts(&self) -> bool {
        false
    }
}
