//! Command-line entrypoints for local runs and the development server.

use std::{net::SocketAddr, str::FromStr};

use axum::serve;
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;

use crate::{
    artifact::{run_scenario, ArtifactStore},
    scenario::{list_scenarios, RunConfig, ScenarioId},
    server::{app, LabServerState},
    LabResult,
};

#[derive(Debug, Parser)]
#[command(name = "picea-lab")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List builtin deterministic scenarios.
    List,
    /// Run one scenario and write artifacts under target/picea-lab/runs.
    Run {
        scenario: String,
        #[arg(long, default_value_t = 120)]
        frames: usize,
    },
    /// Serve the minimal HTTP and SSE protocol.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: SocketAddr,
    },
}

pub async fn run() -> LabResult<()> {
    match Cli::parse().command {
        Command::List => {
            for scenario in list_scenarios() {
                println!("{}\t{}", scenario.id, scenario.name);
            }
            Ok(())
        }
        Command::Run { scenario, frames } => {
            let scenario_id = ScenarioId::from_str(&scenario)?;
            let result = run_scenario(
                &ArtifactStore::default_in_workspace(),
                RunConfig {
                    scenario_id,
                    frame_count: frames,
                    ..RunConfig::default()
                },
            )?;
            println!("{}", result.path.display());
            Ok(())
        }
        Command::Serve { bind } => {
            let listener = TcpListener::bind(bind).await?;
            let state = LabServerState::new(ArtifactStore::default_in_workspace());
            println!("listening on http://{bind}");
            serve(listener, app(state)).await?;
            Ok(())
        }
    }
}
