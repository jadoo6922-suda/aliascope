# Aliascope v0.3

Modular **legal OSINT** CLI (Rust): separate engines, concurrency queue, SQLite cache/history, JSON/CSV/HTML export.

## Engines

| Engine | Capabilities |
|--------|----------------|
| **user** | Alias generation, YAML platforms, concurrent probes, cache, GitHub public API, AI triage |
| **email** | DNS MX/A, Gravatar, Firefox status, optional HIBP names only, manual HIBP link |
| **domain** | DNS A/MX/TXT, crt.sh CT, RDAP |
| **image** | Local file size + MD5 only (no private cloud scraping) |

**Never returns leaked passwords.**

## Install

```bash
git clone https://github.com/jadoo6922-suda/aliascope.git
cd aliascope
cargo build --release
./target/release/aliascope menu
```

Termux: `pkg install rust git clang make` then same.

## Commands

```bash
aliascope user alice --json u.json --csv u.csv --html u.html
aliascope user alice --strict
aliascope email a@b.com
aliascope domain example.com
aliascope image ./photo.jpg
aliascope scan --user alice --email a@b.com --domain example.com -o report
aliascope batch targets.txt --html batch.html
aliascope history
aliascope config
```

### Batch file format

```text
user:alice
email:alice@example.com
domain:example.com
# plain line treated as username
bob
```

## Config files

| Path | Purpose |
|------|---------|
| `data/platforms.yaml` | Profile URL templates (`{user}`) |
| `data/settings.yaml` | rate limit, concurrency, timeout, proxy, cache TTL |
| `~/.aliascope/config.json` | AI + HIBP + GitHub token (local) |
| `~/.aliascope/aliascope.db` | History + HTTP status cache + audit log |

## Architecture

- `engines/*` — isolated OSINT modules  
- `queue` — semaphore-limited concurrent tasks  
- `storage` — SQLite findings, cache, audit  
- `export` — JSON / CSV / HTML  
- `confidence` — high/medium/low/info + reasons  
- `--strict` — high confidence only  

## Ethics

No stealer DBs, no login/2FA bypass, no unauthorized surveillance. Public signals ≠ proof of identity.

## License

MIT
