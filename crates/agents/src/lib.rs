//! # Orbit Agents
//!
//! This crate provides agent capabilities for the Orbit system.

pub use orbit_observability::{
    AgentObservation, AgentObservationKind, AgentObservationSink, AgentRunContext,
    AgentRunObserver, AgentSpan, AgentSpanKind, AgentSpanStatus, MemoryAgentObservationSink,
    MemorySentrySink, NoopAgentObservationSink, NoopSentrySink, Observability,
    ObservabilityBuilder, SentryClient, SentryConfig, SentryEvent, SentryLevel, SentrySink,
    SessionTracerAgentObservationSink,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_exports_observability_surface() {
        let observability = Observability::default();
        let run = observability.start_agent_run(AgentRunContext::new("agent", "run"));

        assert_eq!(run.context().agent_id, "agent");
    }
}
