# rtm - Realtime Markdown Viewer

TUI application for viewing markdown files with live reload. Watches a directory for changes and automatically updates the displayed content while preserving scroll position.

## Install

One-liner:

```bash
curl -fsSL https://raw.githubusercontent.com/aabele/realtime-markdown-viewer/main/install.sh | sh
```

Custom install directory:

```bash
curl -fsSL https://raw.githubusercontent.com/aabele/realtime-markdown-viewer/main/install.sh | INSTALL_DIR=~/.local/bin sh
```

From source:

```bash
git clone https://github.com/aabele/realtime-markdown-viewer.git
cd realtime-markdown-viewer
cargo install --path .
```

Requires Rust toolchain. Install with `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` if missing.

## Usage

```bash
rtm <directory>
```

Examples:

```bash
rtm .planning/
rtm ~/projects/docs/
rtm .
```

## Keybindings

| Key | Action |
|-----|--------|
| j / k | Scroll up/down |
| h / l | Switch between sidebar and content |
| Enter | Open file in tab |
| Tab / Shift+Tab | Switch tabs |
| x | Close tab |
| m | Toggle formatted / syntax highlight |
| t | Toggle Mocha / Latte theme |
| / | Search files by name |
| d / u | Scroll 10 lines |
| g / G | Go to top / bottom |
| ? | Toggle help overlay |
| q / Esc | Quit |

Mouse works everywhere -- click sidebar items, click tabs, scroll with wheel, drag sidebar border to resize.

## Configuration

Create `~/.rtmrc` with TOML syntax:

```toml
# Theme: "mocha" (dark) or "latte" (light)
theme = "mocha"

# Default sidebar width in columns (10-80)
sidebar_width = 30

# Default render mode: "formatted" or "syntax"
render_mode = "formatted"
```

All settings are optional. Defaults are used for missing values.

## Features

- Recursive markdown file discovery
- Two render modes: formatted markdown and syntax highlighted source
- Catppuccin color scheme (Mocha dark + Latte light)
- File watcher with debounced reload (300ms)
- Scroll position preserved on file changes
- Resizable sidebar via mouse drag
- Tab-based file viewing
- Fuzzy file search
- Help overlay on first launch and via `?`

## License

GPL-3.0. See [LICENSE](LICENSE) for details.
