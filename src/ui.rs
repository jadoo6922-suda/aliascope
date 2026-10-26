use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::{config, email_check, username};

pub fn print_banner() {
    let term = Term::stdout();
    let _ = term.write_line("");
    let _ = term.write_line(&format!(
        "{}",
        style("╔══════════════════════════════════════════════════════╗").cyan().bold()
    ));
    let _ = term.write_line(&format!(
        "{}",
        style("║              A L I A S C O P E  v0.1                 ║").cyan().bold()
    ));
    let _ = term.write_line(&format!(
        "{}",
        style("║   Public alias OSINT · Breach presence (no secrets)  ║").cyan()
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
            "Email breach presence check",
            "Configure AI API keys",
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
                    .default(40)
                    .interact_text()?;
                username::run_user_scan(q.trim(), limit).await?;
            }
            1 => {
                let email: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Email address")
                    .interact_text()?;
                email_check::run_email_check(email.trim()).await?;
            }
            2 => config::interactive_config()?,
            3 => print_ethics(),
            _ => break,
        }
        println!();
    }
    Ok(())
}
