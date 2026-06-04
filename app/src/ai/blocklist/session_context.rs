use crate::terminal::model::session::SessionType;

#[derive(Debug, Clone)]
pub struct SessionContext {
    session_type: Option<SessionType>,
    current_working_directory: Option<String>,
}

impl SessionContext {
    pub fn session_type(&self) -> &Option<SessionType> {
        &self.session_type
    }

    pub fn current_working_directory(&self) -> &Option<String> {
        &self.current_working_directory
    }

    pub fn host_id(&self) -> Option<&warp_core::HostId> {
        match &self.session_type {
            Some(SessionType::WarpifiedRemote { host_id }) => host_id.as_ref(),
            Some(SessionType::Local) | None => None,
        }
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        SessionContext {
            session_type: None,
            current_working_directory: None,
        }
    }


}
