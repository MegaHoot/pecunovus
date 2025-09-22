use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;
use crate::node::Node;
use crate::node::NodeConfig;
use crate::node::ServiceHandle;
use tracing_subscriber;

/// CLI for node control.
#[derive(Parser)]
#[clap(name = "pecunovus-node", version)]
pub struct Cli {
    /// Path to data directory
    #[clap(long, default_value = "./data")]
    pub data_dir: PathBuf,

    #[clap(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Initialize the node data directory
    Init {
        /// comma separated bootstrap peers
        #[clap(long)]
        bootstrap: Option<String>,
    },
    /// Run the node
    Run {
        /// network bind address (host:port)
        #[clap(long, default_value = "0.0.0.0:7000")]
        bind: String,

        /// rpc bind address (host:port)
        #[clap(long, default_value = "0.0.0.0:8080")]
        rpc: String,

        /// comma separated bootstrap peers
        #[clap(long)]
        bootstrap: Option<String>,
    },
}

pub async fn run_cli() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Init { bootstrap } => {
            std::fs::create_dir_all(&cli.data_dir)?;
            if let Some(b) = bootstrap {
                std::fs::write(cli.data_dir.join("bootstrap.toml"), format!("peers = [{}]\n", b))?;
            }
            println!("initialized node data dir at {}", cli.data_dir.display());
            Ok(())
        }
        Cmd::Run { bind, rpc, bootstrap } => {
            // load bootstrap peers either from CLI flag or bootstrap.toml
            let bs: Vec<String> = if let Some(b) = bootstrap {
                crate::node::bootstrap::parse_peers_csv(&b)
            } else {
                let cfg_path = cli.data_dir.join("bootstrap.toml");
                if cfg_path.exists() {
                    match crate::node::bootstrap::BootstrapConfig::load(cfg_path) {
                        Ok(cfg) => cfg.peers,
                        Err(_) => vec![],
                    }
                } else {
                    vec![]
                }
            };

            let config = NodeConfig {
                data_dir: cli.data_dir.to_str().unwrap().to_string(),
                bind_addr: bind,
                rpc_addr: rpc,
                bootstrap_peers: bs,
                max_txpool_size: 200_000,
            };

            let node = Node::new(config);
            let svc = node.start().await?;
            // Wait for Ctrl+C
            tokio::signal::ctrl_c().await?;
            println!("Shutting down node...");
            svc.shutdown().await?;
            println!("Node stopped");
            Ok(())
        }
    }
}
