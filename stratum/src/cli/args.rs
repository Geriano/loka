use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "loka-stratum",
    version,
    about = "High-performance Bitcoin Stratum V1 proxy server",
    long_about = "A production-ready Bitcoin Stratum V1 proxy server written in Rust, \
                 featuring lock-free optimizations, comprehensive metrics, and advanced monitoring."
)]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Log format (json, pretty, compact)
    #[arg(long, default_value = "pretty")]
    pub log_format: String,

    /// Enable metrics collection
    #[arg(long, default_value = "true")]
    pub metrics: bool,

    /// Metrics bind address
    #[arg(long, default_value = "0.0.0.0:9090")]
    pub metrics_addr: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the Stratum proxy server
    Start {
        /// Server bind address (overrides config file)
        #[arg(short, long, default_value = "0.0.0.0:3333")]
        bind: String,

        /// Upstream pool address (overrides config file if provided)
        #[arg(short, long)]
        pool: Option<String>,

        /// Maximum concurrent connections (overrides config file if provided)
        #[arg(long)]
        max_connections: Option<usize>,

        /// Connection idle timeout in seconds (overrides config file if provided)
        #[arg(long)]
        idle_timeout: Option<u64>,

        /// Run in daemon mode
        #[arg(short, long)]
        daemon: bool,
    },

    /// Show server status and metrics
    Status {
        /// Metrics endpoint URL
        #[arg(long, default_value = "http://127.0.0.1:9090")]
        endpoint: String,

        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Watch mode (refresh every N seconds)
        #[arg(short, long)]
        watch: Option<u64>,
    },

    /// Validate configuration file
    Config {
        /// Configuration file to validate
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Show effective configuration
        #[arg(long)]
        show: bool,
    },

    /// Generate example configuration file
    Init {
        /// Output file path
        #[arg(short, long, default_value = "loka-stratum.toml")]
        output: PathBuf,

        /// Overwrite existing file
        #[arg(long)]
        force: bool,
    },

    /// Performance benchmarking and testing
    Bench {
        /// Target server address
        #[arg(short, long, default_value = "127.0.0.1:3333")]
        target: String,

        /// Number of concurrent connections
        #[arg(short, long, default_value = "100")]
        connections: usize,

        /// Test duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,

        /// Report format (table, json, csv)
        #[arg(long, default_value = "table")]
        format: String,
    },

    #[cfg(feature = "mock-pool")]
    /// Start a mock mining pool for testing
    MockPool {
        /// Bind address for the mock pool
        #[arg(short, long, default_value = "127.0.0.1:13333")]
        bind: String,

        /// Configuration file for mock pool behavior
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Share acceptance rate (0.0-1.0)
        #[arg(long, default_value = "0.95")]
        accept_rate: f64,

        /// Job rotation interval in seconds
        #[arg(long, default_value = "30")]
        job_interval: u64,

        /// Initial difficulty
        #[arg(long, default_value = "1024")]
        difficulty: u64,

        /// Enable vardiff
        #[arg(long, default_value = "true")]
        vardiff: bool,

        /// Simulated latency in milliseconds
        #[arg(long, default_value = "50")]
        latency: u64,

        /// Error injection rate (0.0-1.0)
        #[arg(long, default_value = "0.01")]
        error_rate: f64,
    },

    #[cfg(feature = "mock-miner")]
    /// Start mock miners for testing and load testing
    MockMiner {
        /// Pool address to connect to
        #[arg(short, long, default_value = "127.0.0.1:3333")]
        pool: String,

        /// Configuration file for miner behavior
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Number of worker connections
        #[arg(short, long, default_value = "1")]
        workers: usize,

        /// Hashrate per worker in MH/s
        #[arg(long, default_value = "100.0")]
        hashrate: f64,

        /// Username for authentication
        #[arg(short, long, default_value = "testuser")]
        username: String,

        /// Password for authentication
        #[arg(long, default_value = "testpass")]
        password: String,

        /// Worker name prefix
        #[arg(long, default_value = "worker")]
        worker_prefix: String,

        /// Share submission interval in seconds
        #[arg(long, default_value = "10.0")]
        share_interval: f64,

        /// Stale share rate (0.0-1.0)
        #[arg(long, default_value = "0.02")]
        stale_rate: f64,

        /// Invalid share rate (0.0-1.0)
        #[arg(long, default_value = "0.01")]
        invalid_rate: f64,

        /// Simulation duration in seconds (0 = infinite)
        #[arg(short, long, default_value = "0")]
        duration: u64,

        /// Enable mining activity logging
        #[arg(long, default_value = "true")]
        log_mining: bool,
    },
}

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
