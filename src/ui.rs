use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::{config, engines, storage};

pub fn print_banner() {
    let t = Term::stdout();
    let _ = t.write_line("");
    let _ = t.write_line(&format!("{}", style("╔══════════════════════════════════════════════════════╗").cyan().bold()));
    let _ = t.write_line(&format!("{}", style("║              A L I A S C O P E  v0.3                 ║").cyan().bold()));
    let _ = t.write_line(&format!("{}", style("║  Engines: user · email · domain · image | queue+DB     ║").cyan()));
    let _ = t.write_line(&format!("{}", style("╚══════════════════════════════════════════════════════╝").cyan().bold()));
    let _ = t.write_line("");
}

pub fn print_ethics() {
    println!("{}", style("Hard boundaries").yellow().bold());
    println!("  • No leaked passwords / stealer DBs");
    println!("  • No login or 2FA bypass");
    println!("  • No silent stalking");
    println!("  • Public data + licensed APIs only");
    println!("  • You are responsible for lawful use");
}

pub async fn run_menu() -> Result<()> {
    print_banner();
    print_ethics();
    loop {
        let items = vec![
            "Username engine",
            "Email engine",
            "Domain engine",
            "Image engine (local file)",
            "History",
            "Config",
            "Ethics",
            "Exit",
        ];
        let sel = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Action")
            .items(&items)
            .interact()?;
        match sel {
            0 => {
                let q: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Username seed")
                    .interact_text()?;
                let _ = engines::username::scan(q.trim(), 40, false).await?;
            }
            1 => {
                let q: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Email")
                    .interact_text()?;
                let _ = engines::email::scan(q.trim(), false).await?;
            }
            2 => {
                let q: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Domain")
                    .interact_text()?;
                let _ = engines::domain::scan(q.trim(), false).await?;
            }
            3 => {
                let q: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Image path")
                    .interact_text()?;
                let _ = engines::image::scan(q.trim(), false).await?;
            }
            4 => {
                if let Ok(st) = storage::Store::open_default() {
                    for (id, kind, query, at, n) in st.recent_scans(20)? {
                        println!("  #{id} [{kind}] {query} findings={n} {at}");
                    }
                }
            }
            5 => config::interactive_config()?,
            6 => print_ethics(),
            _ => break,
        }
        println!();
    }
    Ok(())
}
