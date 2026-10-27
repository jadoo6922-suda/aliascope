use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::path::PathBuf;

use crate::{config, email_check, storage, username};

pub fn print_banner() {
    let term = Term::stdout();
    let _ = term.write_line("");
    let _ = term.write_line(&format!(
        "{}",
        style("╔══════════════════════════════════════════════════════╗").cyan().bold()
    ));
    let _ = term.write_line(&format!(
        "{}",
        style("║              A L I A S C O P E  v0.2                 ║").cyan().bold()
    ));
    let _ = term.write_line(&format!(
        "{}",
        style("║  Public OSINT · SQLite history · JSON/HTML reports   ║").cyan()
    ));
    let _ = term.write_line(&format!(
        "{}",
        style("╚══════════════════════════════════════════════════════╝").cyan().bold()
    ));
    let _ = term.write_line("");
}

pub fn print_ethics() {
    println!("{}", style("Legal & ethical boundaries").yellow().bold());
    println!("  • Only query public profile URLs and licensed breach APIs.");
    println!("  • This tool never returns leaked passwords or private messages.");
    println!("  • Do not use results to harass, stalk, or access accounts.");
    println!("  • Respect site terms, rate limits, and local law.");
    println!("  • You are responsible for how you use the output.");
}

pub async fn run_menu() -> Result<()> {
    print_banner();
    print_ethics();
    println!();

    loop {
        let items = vec![
            "Username / alias discovery",
            "Email free-layer check",
            "Scan history (local DB)",
            "Configure AI / HIBP keys",
            "Show ethics note",
            "Exit",
        ];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an action")
            .items(&items)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let q: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Base username / name fragment")
                    .interact_text()?;
                let limit: usize = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Max candidates")
                    .default("40".into())
                    .interact_text()?
                    .parse()
                    .unwrap_or(40);
                let want_json = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Export JSON report?")
                    .default(true)
                    .interact()?;
                let want_html = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Export HTML report?")
                    .default(true)
                    .interact()?;
                let json_out = if want_json {
                    Some(PathBuf::from(format!("aliascope-{}-report.json", q.trim())))
                } else {
                    None
                };
                let html_out = if want_html {
                    Some(PathBuf::from(format!("aliascope-{}-report.html", q.trim())))
                } else {
                    None
                };
                username::run_user_scan(
                    q.trim(),
                    username::ScanOptions {
                        limit,
                        json_out,
                        html_out,
                        save_db: true,
                    },
                )
                .await?;
            }
            1 => {
                let email: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Email address")
                    .interact_text()?;
                email_check::run_email_check(email.trim()).await?;
            }
            2 => {
                let store = storage::Store::open_default()?;
                let rows = store.recent_scans(20)?;
                if rows.is_empty() {
                    println!("{}", style("No scans stored yet.").yellow());
                } else {
                    println!("{}", style("Recent scans").green().bold());
                    for (id, kind, query, at, hits) in rows {
                        println!("  #{id}  [{kind}]  {query}  hits={hits}  {at}");
                    }
                }
            }
            3 => config::interactive_config()?,
            4 => print_ethics(),
            _ => break,
        }
        println!();
    }
    Ok(())
}
