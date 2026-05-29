mod config;
mod state;

use std::fmt;

pub use config::*;
pub use state::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// The open-source build of Warp.
    Oss,
}

impl Channel {
    /// Whether or not this channel is for internal use only
    pub fn is_dogfood(&self) -> bool {
        false
    }

    /// Whether this channel honors the `--server-root-url` / `--ws-server-url` /
    /// `--session-sharing-server-url` flags (and their `WARP_*` env-var equivalents).
    pub fn allows_server_url_overrides(&self) -> bool {
        false
    }

    /// Returns the CLI command name corresponding to this channel.
    pub fn cli_command_name(&self) -> &'static str {
        "sublight"
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("sublight")
    }
}
