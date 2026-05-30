// Vestigial shared-session state on BlocklistAIController.
// Session sharing is stripped; these fields are never populated (the setters that
// fed them are gone), so the getters always return None. The `response_initiator`
// data-model plumbing they feed is kept as a parked carrier — see plan.md.
use session_sharing_protocol::common::ParticipantId;

use super::BlocklistAIController;

#[derive(Default)]
pub(super) struct SharedSessionState {
    // The participant who initiated the current response stream.
    current_response_initiator: Option<ParticipantId>,
    // The sharer's participant ID.
    sharer_participant_id: Option<ParticipantId>,
}

impl BlocklistAIController {
    pub fn set_current_response_initiator(&mut self, participant_id: ParticipantId) {
        self.shared_session_state.current_response_initiator = Some(participant_id);
    }

    pub(super) fn get_current_response_initiator(&self) -> Option<ParticipantId> {
        self.shared_session_state.current_response_initiator.clone()
    }

    pub(super) fn get_sharer_participant_id(&self) -> Option<ParticipantId> {
        self.shared_session_state.sharer_participant_id.clone()
    }
}
