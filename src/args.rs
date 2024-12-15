// src/args.rs

use clap::{Parser};

/// Command-line arguments for the UDP proxy server
#[derive(Parser, Debug)]
#[command(name = "Mass UDP Proxy Server")]
#[command(author = "Patrick Unick <dev_storm@winux.com>")]
#[command(about = "A simple high-performance UDP proxy server", long_about = None)]
pub struct Cli {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config_file: String,

    /// Number of Tokio worker threads
    #[arg(short, long, value_name = "THREADS", default_value_t = 4)]
    pub threads: usize,
}
