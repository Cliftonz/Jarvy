// Re-export public modules for use by integration tests and external crates
pub mod drift;
pub mod git;
pub mod logging;
pub mod network;
pub mod packages;
pub mod ticket;
pub mod tools;

pub use drift::{DriftConfig, DriftDetector, DriftReport, DriftStatus, EnvironmentState};
pub use logging::{LogError, LogFormat, LogLevel, LogStats, LoggingConfig};
pub use packages::PackagesConfig;
pub use ticket::{TicketData, TicketError, TicketScope};
pub use tools::{add, register_all};
