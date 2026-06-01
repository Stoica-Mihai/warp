use std::sync::LazyLock;

use warpui::keymap::Keystroke;
use warpui::AppContext;

use crate::terminal::TerminalModel;
use crate::util::bindings::keybinding_name_to_keystroke;

pub const ACCEPT_PROMPT_SUGGESTION_KEYBINDING: &str = "terminal:accept_prompt_suggestions";

pub static REJECT_PROMPT_SUGGESTION_KEYSTROKE: LazyLock<Keystroke> = LazyLock::new(|| Keystroke {
    ctrl: true,
    key: "c".to_owned(),
    ..Default::default()
});

pub fn is_accept_prompt_suggestion_bound_to_cmd_enter(app: &AppContext) -> bool {
    static CMD_ENTER_KEYSTROKE: LazyLock<Keystroke> = LazyLock::new(|| Keystroke {
        cmd: true,
        key: "enter".to_owned(),
        ..Default::default()
    });
    keybinding_name_to_keystroke(ACCEPT_PROMPT_SUGGESTION_KEYBINDING, app)
        .is_some_and(|keystroke| keystroke == *CMD_ENTER_KEYSTROKE)
}

pub fn is_accept_prompt_suggestion_bound_to_ctrl_enter(app: &AppContext) -> bool {
    static CTRL_ENTER_KEYSTROKE: LazyLock<Keystroke> = LazyLock::new(|| Keystroke {
        ctrl: true,
        key: "enter".to_owned(),
        ..Default::default()
    });
    keybinding_name_to_keystroke(ACCEPT_PROMPT_SUGGESTION_KEYBINDING, app)
        .is_some_and(|keystroke| keystroke == *CTRL_ENTER_KEYSTROKE)
}

/// Returns `true` if the last AI block in the blocklist has a pending suggested diff or unit test
/// suggestion.
pub fn has_pending_code_or_unit_test_prompt_suggestion(
    terminal_model: &TerminalModel,
    app: &AppContext,
) -> bool {
    false
}
