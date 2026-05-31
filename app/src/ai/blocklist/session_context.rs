use warpui::AppContext;

use crate::terminal::model::session::active_session::ActiveSession;
use crate::terminal::model::session::SessionType;
use crate::terminal::ShellLaunchData;

#[derive(Debug, Clone)]
pub struct SessionContext {
    session_type: Option<SessionType>,
    shell: Option<ShellLaunchData>,
    current_working_directory: Option<String>,
}

impl SessionContext {
    pub fn from_session(session: &ActiveSession, app: &AppContext) -> Self {
        SessionContext {
            session_type: session.session_type(app),
            shell: session.shell_launch_data(app),
            current_working_directory: session.current_working_directory().cloned(),
        }
    }

    pub fn session_type(&self) -> &Option<SessionType> {
        &self.session_type
    }

    pub fn shell(&self) -> &Option<ShellLaunchData> {
        &self.shell
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

    pub fn is_remote(&self) -> bool {
        matches!(self.session_type, Some(SessionType::WarpifiedRemote { .. }))
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        SessionContext {
            session_type: None,
            shell: None,
            current_working_directory: None,
        }
    }

    #[cfg(test)]
    pub fn new_with_session_type_for_test(session_type: Option<SessionType>) -> Self {
        SessionContext {
            session_type,
            shell: None,
            current_working_directory: None,
        }
    }
}
