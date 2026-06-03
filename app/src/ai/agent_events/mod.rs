//! Shared agent-event stream utilities used by orchestration consumers and
//! third-party harness bridges.

mod driver;
mod message_hydrator;

#[cfg(test)]
pub(crate) use driver::{
    agent_event_backoff, agent_event_failures_exceeded_threshold, AgentEventConsumer,
    AgentEventConsumerControlFlow, AgentEventDriverConfig, AgentEventDriverState,
    AgentEventSource, AgentEventSourceItem, run_agent_event_driver,
    DEFAULT_AGENT_EVENT_FAILURES_BEFORE_ERROR_LOG,
    DEFAULT_AGENT_EVENT_RECONNECT_BACKOFF_STEPS, DEFAULT_PERMANENT_ERROR_BACKOFF_STEPS,
};


#[cfg(test)]
pub(crate) use message_hydrator::MessageHydrator;

#[cfg(test)]
mod driver_tests;
#[cfg(test)]
mod message_hydrator_tests;
