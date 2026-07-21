# cdp_rs - Browser Automation Framework

## Module Structure

```
cdp_rs/
├── src/
│   ├── lib.rs              # Core library
│   ├── main.rs             # Main binary (cdp_rs)
│   ├── bin/
│   │   ├── arxiv.rs        # arXiv tools
│   │   ├── vixra.rs        # viXra.org tools
│   │   └── endorse.rs      # Endorsement system
│   ├── arxiv/              # arXiv module
│   │   ├── mod.rs
│   │   ├── submit.rs       # arXiv submission
│   │   ├── codes.rs        # Endorsement codes
│   │   ├── endorsers.rs    # Find endorsers
│   │   └── send.rs         # Send emails
│   ├── browser.rs          # Browser management
│   ├── page.rs             # Page interactions
│   ├── upload.rs           # File uploads
│   └── ...                 # Other core modules
```

## Binaries

| Binary | Description |
|--------|-------------|
| `cdp_rs` | Core browser automation |
| `arxiv` | arXiv submission and endorsement |
| `vixra` | viXra.org submission |
| `endorse` | Endorsement system |

## Usage

```bash
# Core browser automation
cdp_rs page <url> [screenshot]
cdp_rs screenshot <url> <output>
cdp_rs pdf <url> <output>

# arXiv tools
arxiv submit       # Submit paper to arXiv
arxiv codes        # Extract endorsement codes
arxiv endorsers    # Find endorsers
arxiv send         # Send endorsement emails

# viXra.org tools
vixra submit       # Submit paper to viXra.org

# Endorsement system
endorse find       # Find endorsers
endorse send       # Send emails
endorse status     # Check status
```

## Development

```bash
# Build all
cargo build --release

# Build specific binary
cargo build --release --bin arxiv
cargo build --release --bin vixra
cargo build --release --bin endorse

# Run tests
cargo test
```

## Modules

### arxiv Module
- `submit.rs` - arXiv paper submission
- `codes.rs` - Extract endorsement codes from Gmail
- `endorsers.rs` - Find potential endorsers
- `send.rs` - Send endorsement emails

### Core Modules
- `browser.rs` - Browser management
- `page.rs` - Page interactions
- `upload.rs` - File uploads
- `network.rs` - Network interception
- `screenshot.rs` - Screenshot capture
- `pdf.rs` - PDF generation

---
*cdp_rs v3.0 - Modular browser automation*
