pub mod llm_judge;
mod step;
mod user_defaults;
mod util;
use std::fs::File;
use std::io::Write;

pub use step::*;
pub use user_defaults::*;
pub use util::*;
use warpui::integration::PersistedDataMap;
pub use warpui::integration::RUNTIME_TAG_FAILURE_REASON;
use warpui::{App, WindowId};

pub use crate::terminal::view::agent_view_state::AgentViewState;

pub const TOTAL_REQUEST_COST_PREFIX: &str = "Total request cost: ";
pub const TOTAL_EXCHANGES_PREFIX: &str = "Total number of exchanges: ";
pub const TOTAL_TOKEN_USAGE_PREFIX: &str = "Total token usage: ";

pub const RUNTIME_TAG_TOTAL_REQUEST_COST: &str = "total_request_cost";
pub const RUNTIME_TAG_TOTAL_EXCHANGES: &str = "total_exchanges";
pub const RUNTIME_TAG_TOKEN_USAGE_PREFIX: &str = "token_usage.";

const CODE_DIFF_OUTPUT_FILE_ENV_VAR: &str = "CODE_DIFF_OUTPUT_FILE";

pub fn output_code_diff_with_base_commit(
    base_commit: &str,
    working_dir: &str,
    test_files_str: &str,
) {
    use command::blocking::Command;

    let Some(mut output_file) = open_debug_file_from_env(CODE_DIFF_OUTPUT_FILE_ENV_VAR) else {
        log::error!("Could not open debug file from env");
        return;
    };
    // Clear the test files from the diff, because we are not interested in seeing those.
    log::debug!(
        "[GIT OPERATION] mod.rs output_code_diff_with_base_commit git checkout {base_commit} -- {test_files_str}"
    );
    let _ = Command::new("git")
        .args(["checkout", base_commit, "--", test_files_str])
        .current_dir(working_dir)
        .output();
    log::debug!(
        "[GIT OPERATION] mod.rs output_code_diff_with_base_commit git --no-pager diff {base_commit}"
    );
    let git_diff_output = Command::new("git")
        .args(["--no-pager", "diff", base_commit])
        .current_dir(working_dir)
        .output();
    write_git_diff_output_to_file(
        git_diff_output,
        &mut output_file,
        &std::env::var(CODE_DIFF_OUTPUT_FILE_ENV_VAR)
            .expect("Could not find diff output file env var"),
    )
}

pub fn output_code_diff_debug_info(_app: &mut App, _window_id: WindowId) {}

fn write_git_diff_output_to_file(
    diff_output: std::io::Result<std::process::Output>,
    file: &mut File,
    file_name: &str,
) {
    match diff_output {
        Ok(output) if output.status.success() => {
            let diff = String::from_utf8_lossy(&output.stdout);
            writeln!(file, "Diff for file: {file_name}\n{diff}\n")
                .expect("Failed to write diff to code diff file");
        }
        Ok(output) => {
            let err = String::from_utf8_lossy(&output.stderr);
            writeln!(
                file,
                "Failed to get diff for file: {file_name}\nGit error:\n{err}\n"
            )
            .expect("Failed to write error to code diff file");
        }
        Err(e) => {
            writeln!(
                file,
                "Failed to run git diff for file: {file_name}\nError: {e}\n"
            )
            .expect("Failed to write command error to code diff file");
        }
    }
}

pub fn output_conversation_debug_info(
    _app: &mut App,
    _window_id: WindowId,
    _persisted_data: &mut PersistedDataMap,
) {
}

// Get debug output file path from environment
pub fn open_debug_file_from_env(env_var: &str) -> Option<File> {
    let file_path = std::env::var(env_var).ok();
    if let Some(file_path) = &file_path {
        // Clear the file if it exists
        let _ = std::fs::remove_file(file_path);
        Some(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)
                .expect("Failed to open debug output file"),
        )
    } else {
        None
    }
}
