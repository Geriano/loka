#[cfg(feature = "mock-pool")]
pub mod config;
#[cfg(feature = "mock-pool")]
pub mod difficulty;
#[cfg(feature = "mock-pool")]
pub mod job_manager;
#[cfg(feature = "mock-pool")]
pub mod pool;
#[cfg(feature = "mock-pool")]
pub mod responses;
#[cfg(feature = "mock-pool")]
pub mod validator;

#[cfg(feature = "mock-pool")]
pub use config::MockConfig;
#[cfg(feature = "mock-pool")]
pub use pool::MockPool;