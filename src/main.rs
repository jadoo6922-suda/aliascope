mod ai;
mod confidence;
mod config;
mod engines;
mod export;
mod platforms;
mod queue;
mod storage;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;

use crate::confidence::Finding;

#[derive(Parser, Debug)]
#[command(name = "aliascope")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Aliascope — modular legal OSINT (user, email, domain, image)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Menu,
    /// Username / alias public discovery
    User {
        query: String,
        #[arg(short, long, default_value_t = 40)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        strict: bool,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long)]
        csv: Option<PathBuf>,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Email free-layer + DNS/MX (+ optional HIBP)
    Email {
        address: String,
        #[arg(long, default_value_t = false)]
        strict: bool,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long)]
        csv: Option<PathBuf>,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Domain DNS / CT / RDAP
    Domain {
        name: String,
        #[arg(long, default_value_t = false)]
        strict: bool,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long)]
        csv: Option<PathBuf>,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Local image file hashes only
    Image {
        path: String,
        #[arg(long, default_value_t = false)]
        strict: bool,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long)]
        csv: Option<PathBuf>,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Unified scan shortcut
    Scan {
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        image: Option<String>,
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// Batch file: lines like user:alice / email:a@b.com / domain:example.com
    Batch {
        file: PathBuf,
        #[arg(long, default_value_t = false)]
        strict: bool,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    History {
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    Config,
    Ethics,
}

fn emit(
    kind: &str,
    query: &str,
    findings: &[Finding],
    json: Option<PathBuf>,
    csv: Option<PathBuf>,
    html: Option<PathBuf>,
) -> Result<()> {
    export::export_all(
        kind,
        query,
        findings,
        json.as_deref(),
        csv.as_deref(),
        html.as_deref(),
    )?;
    if json.is_some() || csv.is_some() || html.is_some() {
        println!("{}", style("Reports written.").green());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Menu) {
        Commands::Menu => ui::run_menu().await?,
        Commands::User {
            query,
            limit,
            strict,
            json,
            csv,
            html,
        } => {
            ui::print_banner();
            let f = engines::username::scan(&query, limit, strict).await?;
            emit("user", &query, &f, json, csv, html)?;
        }
        Commands::Email {
            address,
            strict,
            json,
            csv,
            html,
        } => {
            ui::print_banner();
            let f = engines::email::scan(&address, strict).await?;
            emit("email", &address, &f, json, csv, html)?;
        }
        Commands::Domain {
            name,
            strict,
            json,
            csv,
            html,
        } => {
            ui::print_banner();
            let f = engines::domain::scan(&name, strict).await?;
            emit("domain", &name, &f, json, csv, html)?;
        }
        Commands::Image {
            path,
            strict,
            json,
            csv,
            html,
        } => {
            ui::print_banner();
            let f = engines::image::scan(&path, strict).await?;
            emit("image", &path, &f, json, csv, html)?;
        }
        Commands::Scan {
            user,
            email,
            domain,
            image,
            out,
            strict,
        } => {
            ui::print_banner();
            let mut all = Vec::new();
            let mut label = Vec::new();
            if let Some(u) = user {
                all.extend(engines::username::scan(&u, 40, strict).await?);
                label.push(format!("user:{u}"));
            }
            if let Some(e) = email {
                all.extend(engines::email::scan(&e, strict).await?);
                label.push(format!("email:{e}"));
            }
            if let Some(d) = domain {
                all.extend(engines::domain::scan(&d, strict).await?);
                label.push(format!("domain:{d}"));
            }
            if let Some(img) = image {
                all.extend(engines::image::scan(&img, strict).await?);
                label.push(format!("image:{img}"));
            }
            let q = label.join(" | ");
            if let Some(base) = out {
                let html = base.with_extension("html");
                let json = base.with_extension("json");
                let csv = base.with_extension("csv");
                emit("scan", &q, &all, Some(json), Some(csv), Some(html))?;
            }
        }
        Commands::Batch { file, strict, json, html } => {
            ui::print_banner();
            let text = std::fs::read_to_string(file)?;
            let mut all = Vec::new();
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(rest) = line.strip_prefix("user:") {
                    all.extend(engines::username::scan(rest.trim(), 30, strict).await?);
                } else if let Some(rest) = line.strip_prefix("email:") {
                    all.extend(engines::email::scan(rest.trim(), strict).await?);
                } else if let Some(rest) = line.strip_prefix("domain:") {
                    all.extend(engines::domain::scan(rest.trim(), strict).await?);
                } else if let Some(rest) = line.strip_prefix("image:") {
                    all.extend(engines::image::scan(rest.trim(), strict).await?);
                } else {
                    // default username
                    all.extend(engines::username::scan(line, 30, strict).await?);
                }
            }
            emit("batch", "batch", &all, json, None, html)?;
        }
        Commands::History { limit } => {
            ui::print_banner();
            let store = storage::Store::open_default()?;
            let rows = store.recent_scans(limit)?;
            if rows.is_empty() {
                println!("{}", style("No scans yet.").yellow());
            } else {
                for (id, kind, query, at, n) in rows {
                    println!("  #{id} [{kind}] {query} findings={n} {at}");
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
        style("Authorized use only. Public signals ≠ identity proof.").dim()
    );
    Ok(())
}
