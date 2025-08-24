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
pub enum DatabaseCommands {
    /// Run database migrations
    Migrate {
        /// Run up migrations
        #[arg(long)]
        up: bool,

        /// Run down migrations (specify number of migrations to rollback)
        #[arg(long)]
        down: Option<u32>,
    },

    /// Check database connection and status
    Status,

    /// Reset database (drop all tables and run fresh migrations)
    Reset {
        /// Confirm the reset operation
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the Stratum proxy server
    Start {
        /// Server bind address (overrides config file)
        #[arg(short, long, default_value = "3333")]
        bind: u16,

        /// Database connection string
        #[arg(long = "db", env = "DATABASE_URL")]
        database_url: String,

        /// Maximum concurrent connections (overrides config file if provided)
        #[arg(long, default_value = "1000")]
        max_connections: usize,

        /// Connection idle timeout in seconds (overrides config file if provided)
        #[arg(long, default_value = "60")]
        idle_timeout: u64,

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

    /// Database management commands
    Database {
        /// Database url connection string
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,

        #[command(subcommand)]
        command: DatabaseCommands,
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
}

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
