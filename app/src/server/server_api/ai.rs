// Re-export ambient agent types for backwards compatibility
pub use crate::ai::ambient_agents::{AgentConfigSnapshot, AgentSource};

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
