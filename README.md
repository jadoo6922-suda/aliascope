# Aliascope

**Legal OSINT CLI** (Rust) for Termux, Linux, and macOS.

| Module | What it does | Cost |
|--------|----------------|------|
| **Username / alias discovery** | Expands a handle into likely variants and probes *public* profile URLs | Free |
| **Platforms YAML** | Add/remove sites without recompiling (`data/platforms.yaml`) | Free |
| **Local SQLite history** | Every user scan stored under `~/.aliascope/aliascope.db` | Free |
| **JSON / HTML reports** | Export scan results for sharing or archiving | Free |
| **Email free layer** | Registration / avatar signals (Gravatar, Firefox Accounts, …) | Free |
| **Email HIBP layer** | Breach *names* only via official API | Optional paid key |
| **AI agents** | Optional dual API keys for alias ideas + neutral summary | Your keys |

**This tool never returns leaked passwords.**

---

## Ethics

- Investigate only what you are **authorized** to investigate.
- No stalking, harassment, or account takeover.
- Public checks and licensed APIs only.
- Free email layer reports **signals**, not proof of identity.
- HIBP path reports **breach presence + names**, never passwords.

---

## Install

### Termux (Android)

```bash
pkg update && pkg install rust git clang make
git clone https://github.com/jadoo6922-suda/aliascope.git
cd aliascope
cargo build --release
./target/release/aliascope menu
```

```bash
cp target/release/aliascope $PREFIX/bin/   # optional
```

### Linux / macOS

```bash
git clone https://github.com/jadoo6922-suda/aliascope.git
cd aliascope
cargo build --release
./target/release/aliascope menu
```

---

## Usage

```text
aliascope menu
aliascope user alice
aliascope user alice --limit 50 --json out.json --html out.html
aliascope user alice --no-db
aliascope email a@b.com
aliascope history
aliascope config
aliascope ethics
```

### Reports

```bash
aliascope user target --json report.json --html report.html
```

- JSON: machine-readable archive  
- HTML: dark readable report for browser  
- DB: `~/.aliascope/aliascope.db` (unless `--no-db`)

### Custom platforms

Edit or copy:

```text
data/platforms.yaml
# or
~/.aliascope/platforms.yaml
```

Example entry:

```yaml
platforms:
  - name: GitHub
    url: "https://github.com/{user}"
    enabled: true
```

`{user}` is replaced with each alias candidate.

---

## Configure (optional)

```bash
aliascope config
```

| Field | Purpose |
|--------|---------|
| Primary AI key | Groq / OpenAI-compatible |
| Secondary AI key | Gemini |
| HIBP API key | Optional paid breach-name lookup |

Keys stay in `~/.aliascope/config.json` — never commit them.

---

## Project layout

```text
data/platforms.yaml   editable platform list
src/
  main.rs             CLI
  ui.rs               interactive menu
  username.rs         alias engine + probes
  platforms.rs        YAML loader
  email_check.rs      free email layer + optional HIBP
  storage.rs          SQLite history
  export.rs           JSON / HTML writers
  ai.rs               dual-agent helpers
  config.rs           local keys
```

---

## Roadmap (next)

- DNS / MX checks for emails  
- Stronger confidence scoring  
- Batch scans from a file  
- GitHub Actions release binaries  
- TUI live progress  

---

## License

MIT — see [LICENSE](LICENSE).

## Disclaimer

Authors are not responsible for misuse. OSINT output can be wrong; verify manually before any action.
