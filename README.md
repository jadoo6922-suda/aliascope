# Aliascope

**Legal OSINT CLI** (Rust) for Termux, Linux, and macOS.

| Module | What it does | Cost |
|--------|----------------|------|
| **Username / alias discovery** | Expands a handle into likely variants and probes *public* profile URLs | Free |
| **Email free layer** | Registration / avatar signals (Gravatar, Firefox Accounts, cautious probes) | **Free** |
| **Email HIBP layer** | Breach *names* only via official API | Optional paid HIBP key |
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

Optional PATH install:

```bash
cp target/release/aliascope $PREFIX/bin/
```

### Linux / macOS

```bash
# Rust: https://rustup.rs
git clone https://github.com/jadoo6922-suda/aliascope.git
cd aliascope
cargo build --release
./target/release/aliascope menu
```

---

## Usage

```text
aliascope menu              Interactive UI (recommended)
aliascope user alice        Alias / public profile scan
aliascope email a@b.com     Free registration signals (+ optional HIBP)
aliascope config            Store AI / HIBP keys locally
aliascope ethics            Legal boundaries
```

### Configure (optional)

```bash
aliascope config
```

Writes `~/.aliascope/config.json` **on your device only**:

| Field | Purpose |
|--------|---------|
| Primary AI key | Groq / OpenAI-compatible |
| Secondary AI key | Gemini |
| HIBP API key | Optional paid breach-name lookup |

Never commit real keys to git.

---

## Email checks explained

### Free (default)

1. **Gravatar** — public avatar for MD5(email)  
2. **Firefox Accounts** — public status API when available  
3. **Cautious form probes** — ambiguous HTTP signals only (labeled low/medium confidence)  
4. **Manual HIBP link** — open the free website check in a browser  

### Optional HIBP API

If you buy a key from [Have I Been Pwned API](https://haveibeenpwned.com/API/Key) and save it via `aliascope config`, the tool can list **breach names/dates**. Still **no passwords**.

---

## Username discovery

- Deterministic variants (suffixes, years, light leet, prefixes)
- Optional AI expansion if a primary key is set
- Platforms include GitHub, GitLab, Reddit, X, Instagram, TikTok, YouTube, LinkedIn, and more
- Rate-limited (~120ms between requests)
- HTTP 200/3xx = *possible* public page, not identity proof

---

## Project layout

```text
src/
  main.rs          CLI entry
  ui.rs            Banner + interactive menu
  username.rs      Alias engine + probes
  platforms.rs     Public profile URL templates
  email_check.rs   Free layer + optional HIBP
  ai.rs            Dual-agent helpers
  config.rs        Local keys
```

---

## Build

Release profile is size-optimized (`lto`, `strip`, `opt-level = "z"`).

```bash
cargo build --release
```

---

## License

MIT — see [LICENSE](LICENSE).

## Disclaimer

Authors are not responsible for misuse. OSINT output can be wrong; verify manually before any action.
