# Explanation

Understanding-oriented documentation that illuminates the "why" behind Arawn's design.

These pages provide context, background, and reasoning for the decisions that shaped the system. They are meant to help you build a mental model of how Arawn works and why it works that way.

## Contents

- [Architecture](architecture.md) -- The big picture: design philosophy, crate layers, data flow, and the reasoning behind key technology choices.

- [Security Model](security-model.md) -- Defense-in-depth: how eight layers of security work together to protect the system from authentication through to secret management.

- [Memory & Knowledge Graph](memory-and-knowledge.md) -- How persistent memory, vector search, knowledge graphs, and confidence scoring combine to give the agent recall across sessions.

- [Sessions & Workstreams](sessions-and-workstreams.md) -- The organizational model: how sessions, workstreams, and filesystem isolation create structured, persistent conversation contexts.

- [Tool Execution Pipeline](tool-execution.md) -- The complete journey of a tool call from the agent loop through secret resolution, gate enforcement, sandboxing, and output sanitization.

- [Plugins & Hooks](plugins-and-hooks.md) -- How the plugin system extends Arawn with skills, agents, and lifecycle hooks while maintaining security boundaries.
