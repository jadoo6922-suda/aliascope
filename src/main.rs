mod ai;
mod config;
mod email_check;
mod export;
mod platforms;
mod storage;
mod ui;
mod username;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "aliascope")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Aliascope — legal OSINT: public username discovery + free email signals")]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactive menu (recommended)
    Menu,
    /// Discover likely usernames / public profiles
    User {
        /// Base handle to expand and probe
        query: String,
        /// Max alias candidates to generate
        #[arg(short, long, default_value_t = 40)]
        limit: usize,
        /// Write JSON report to this path
        #[arg(long)]
        json: Option<PathBuf>,
        /// Write HTML report to this path
        #[arg(long)]
        html: Option<PathBuf>,
        /// Skip writing results to local SQLite DB
        #[arg(long, default_value_t = false)]
        no_db: bool,
    },
    /// Free email registration signals (+ optional HIBP)
    Email {
        address: String,
    },
    /// Show recent scans from local DB
    History {
        #[arg(short, long, default_value_t = 15)]
        limit: usize,
    },
    /// Configure AI / HIBP keys
    Config,
    /// Show ethics / legal boundaries
    Ethics,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Menu) {
        Commands::Menu => ui::run_menu().await?,
        Commands::User {
            query,
            limit,
            json,
            html,
            no_db,
        } => {
            ui::print_banner();
            username::run_user_scan(
                &query,
                username::ScanOptions {
                    limit,
                    json_out: json,
                    html_out: html,
                    save_db: !no_db,
                },
            )
            .await?;
        }
        Commands::Email { address } => {
            ui::print_banner();
            email_check::run_email_check(&address).await?;
        }
        Commands::History { limit } => {
            ui::print_banner();
            let store = storage::Store::open_default()?;
            let rows = store.recent_scans(limit)?;
            if rows.is_empty() {
                println!("{}", style("No scans stored yet.").yellow());
            } else {
                println!("{}", style("Recent scans").green().bold());
                for (id, kind, query, at, hits) in rows {
                    println!("  #{id}  [{kind}]  {query}  hits={hits}  {at}");
                }
            }
        }
        Commands::Config => {
            ui::print_banner();
            config::interactive_config()?;
        }
        Commands::Ethics => {
            ui::print_banner();
            ui::print_ethics();
        }
    }
    println!();
    println!(
        "{}",
        style("Done. Use results only on assets you are authorized to investigate.").dim()
    );
    Ok(())
}
