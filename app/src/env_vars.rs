use itertools::Itertools;
use serde::{Deserialize, Serialize};
use warp_util::path::ShellFamily;

use crate::external_secrets::ExternalSecret;
use crate::terminal::shell::ShellType;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnvVarSecretCommand {
    pub command: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EnvVarValue {
    Constant(String),
    Command(EnvVarSecretCommand),
    Secret(ExternalSecret),
}

impl Default for EnvVarValue {
    fn default() -> Self {
        EnvVarValue::Constant(String::new())
    }
}

pub fn serialize_variables_for_shell<'s, I: IntoIterator<Item = (&'s str, &'s EnvVarValue)>>(
    pairs: I,
    shell_type: ShellType,
) -> String {
    match shell_type {
        ShellType::Fish => {
            serialize_variables_internal(pairs, "set -x ", " ", ";", " ", shell_type.into())
        }
        ShellType::Bash | ShellType::Zsh => {
            serialize_variables_internal(pairs, "", "=", "", " ", shell_type.into())
        }
        ShellType::PowerShell => {
            serialize_variables_internal(pairs, "$env:", " = ", ";", " ", shell_type.into())
        }
    }
}

fn get_init_command_for_env_var(value: &EnvVarValue, shell_family: ShellFamily) -> String {
    match value {
        EnvVarValue::Constant(val) => match shell_family {
            ShellFamily::Posix => shell_family.escape(val).into_owned(),
            ShellFamily::PowerShell => format!("'{}'", val.replace("'", "''")),
        },
        EnvVarValue::Command(cmd) => format!("$({})", cmd.command),
        EnvVarValue::Secret(secret) => {
            format!("$({})", secret.get_secret_extraction_command(shell_family))
        }
    }
}

fn serialize_variables_internal<'s, I: IntoIterator<Item = (&'s str, &'s EnvVarValue)>>(
    pairs: I,
    prefix: &str,
    separator: &str,
    postfix: &str,
    delimiter: &str,
    shell_family: ShellFamily,
) -> String {
    pairs
        .into_iter()
        .map(|(name, value)| {
            format!(
                "{}{}{}{}{}",
                prefix,
                shell_family.escape(name),
                separator,
                get_init_command_for_env_var(value, shell_family),
                postfix
            )
        })
        .collect_vec()
        .join(delimiter)
}
