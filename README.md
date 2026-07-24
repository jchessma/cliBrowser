# clibrowser

A full-featured CLI web browser in Rust — picking up where Lynx and Links left
off, with modern web support (CSS, JavaScript, cookies, forms).

It renders web pages as flowed text in your terminal, lets you navigate links
and form fields with the keyboard, and supports pluggable JavaScript backends —
from a sandboxed in-process QuickJS engine to a headless Chrome backend for
sites that need a full DOM.

## Features

- **Text-mode rendering** — pages are laid out as wrapped blocks, headings,
  lists, links, and form fields, with bold/inline styling preserved.
- **Keyboard-driven navigation** — Vim-style keys, Tab to move between links and
  form fields, Enter to follow or submit.
- **Forms** — type into text fields, toggle checkboxes/radios, and submit forms
  via GET or POST. Focusing a text field enters edit mode automatically
  (Lynx-style).
- **Pluggable JavaScript** — run JS in-process via QuickJS (default), drive a
  headless Chrome backend for heavy sites, or disable JS entirely.
- **CSS support** — inline and stylesheet CSS are parsed; `display: none`,
  `noscript` visibility, and basic text styling are honored.
- **History, bookmarks & cookies** — back/forward navigation, a persistent
  bookmark store, session cookie handling, and a browsing history.
- **Configurable** — CLI flags select the JS backend, log level, and startup URL.

## Install

From source (requires the Rust toolchain):

```sh
cargo install --path .
```

Or build and run directly:

```sh
cargo build --release
./target/release/clibrowser https://example.com
```

## Usage

```sh
clibrowser [URL] [OPTIONS]
```

| Flag | Description |
| --- | --- |
| `[URL]` | URL to open on startup (defaults to `about:home`) |
| `--chrome` | Use the headless Chrome backend for JavaScript (requires Chrome installed) |
| `--no-js` | Disable JavaScript execution entirely |
| `--log <LEVEL>` | Log level: `error`, `warn`, `info`, `debug`, `trace` (default: `warn`) |

### Examples

```sh
clibrowser https://en.wikipedia.org/wiki/Terminal_emulator
clibrowser https://example.com --no-js
clibrowser https://example.com --chrome --log debug
```

## Keybindings

| Key | Action |
| --- | --- |
| `q` / `Ctrl-C` | Quit |
| `j` / `↓` | Scroll down one line |
| `k` / `↑` | Scroll up one line |
| `d` | Scroll down 10 lines |
| `u` | Scroll up 10 lines |
| `PageDown` / `PageUp` | Page down / up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Tab` / `Shift-Tab` | Next / previous link or form field |
| `Enter` | Follow focused link, or submit the focused form |
| `Space` | Toggle checkbox / radio when a field is focused |
| `H` / `Alt-←` / `Backspace` | Go back |
| `L` / `Alt-→` | Go forward |
| `o` | Open a URL |
| `r` / `F5` | Reload |
| `b` | Bookmark the current page |
| `B` | Show bookmarks |
| `h` | Show history |
| `y` | Copy the focused link's URL |
| `?` | Show help |

When a text field is focused, typing edits its value; `Enter` submits the
containing form and `Backspace` deletes a character (in non-edit mode
`Backspace` navigates back).

## Architecture

```
src/
├── main.rs            CLI entry point (clap args, JS backend selection)
├── network/           HTTP client (reqwest), cookies, response handling
├── dom/               HTML parsing and DOM tree (scraper/ego-tree)
├── css/               CSS parsing (lightningcss) and style resolution
├── js/                Pluggable JS backends: QuickJS, Chrome, or None
├── layout/            Block-based text flow and wrapping (textwrap)
├── renderer/          Terminal rendering via ratatui
├── browser/           State: page model, history, bookmarks
└── ui/                Event loop, keybindings, app controller
```

### Feature flags

| Feature | Default | Description |
| --- | --- | --- |
| `quickjs-engine` | yes | In-process JavaScript via `rquickjs` |
| `chrome-engine` | no | Headless Chrome JavaScript via `chromiumoxide` |

Build without the default engine, or with Chrome:

```sh
cargo build --no-default-features --features quickjs-engine
cargo build --features chrome-engine
```

## Testing

```sh
cargo test
```

## License

Licensed under the [GNU General Public License v2.0](LICENSE) — the same license
used by Lynx and Links.
