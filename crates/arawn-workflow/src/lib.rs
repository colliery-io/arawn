//! Workflow engine integration — embeds cloacina's DefaultRunner for
//! scheduled agent workflows with DAG execution, cron scheduling, and
//! hot-loaded .cloacina packages.

pub mod agent_executor;
pub mod runner;
pub mod scaffold;
pub mod tools;

pub use agent_executor::DecisionService;
pub use runner::WorkflowRunner;
pub use tools::{
    SharedWorkflowRunner, WorkflowCreateTool, WorkflowDeleteTool, WorkflowListTool,
    WorkflowStatusTool,
};
