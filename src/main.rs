mod ai;
mod config;
mod email_check;
mod platforms;
mod ui;
mod username;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;

#[derive(Parser, Debug)]
#[command(name = "aliascope")]
#[command(version = "0.1.0")]
#[command(about = "Aliascope — legal OSINT: public username discovery + breach presence checks")]
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
    },
    /// Check whether an email appears in known breaches (NO passwords returned)
    Email {
        /// Email address to check
        address: String,
    },
    /// Configure AI API keys (stored locally under ~/.aliascope)
    Config,
    /// Show ethics / legal boundaries
    Ethics,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Menu) {
        Commands::Menu => ui::run_menu().await?,
        Commands::User { query, limit } => {
            ui::print_banner();
            username::run_user_scan(&query, limit).await?;
        }
        Commands::Email { address } => {
            ui::print_banner();
            email_check::run_email_check(&address).await?;
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
        style("Done. Use results only on assets you are authorized to investigate.")
            .dim()
    );
    Ok(())
}
