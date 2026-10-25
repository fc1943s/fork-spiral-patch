# Spiral for the Fork git client

Spiral syntax highlighting in Fork's diff and file views, and diff fonts smaller than Fork's minimum of 10.

```powershell
pwsh install.ps1                  # Fork from scoop, closed
pwsh install.ps1 -Restart         # closes Fork, installs, reopens it
pwsh install.ps1 -FontSize 7      # also sets the diff font size
pwsh install.ps1 -ForkDir <dir>   # a Fork that scoop doesn't manage
```

Run it again after every `scoop update fork`. Each run rebuilds from the originals it saved, so it can run any number
of times. It needs Rust, the .NET 10 SDK and `deps/spiral`, which `pwsh scripts/init.ps1` links to a spiral checkout
next to this one (cloning [i574n/spiral](https://github.com/i574n/spiral) there if there is none). Its `lib/spiral` is
the tokenizer's test corpus.

## Testing against a Fork release

```powershell
pwsh test-fork.ps1                  # newest release in Fork's update feed
pwsh test-fork.ps1 -Version 2.15.3  # a specific release
```

`test-fork.ps1` downloads the release from Fork's update CDN, checks its SHA-1 and extracts it to a temporary folder.
It runs `install.ps1` on it twice (the second run checks reinstalling), then starts that Fork and waits until it has
loaded both the proxy and `biturbo_original.dll` from its own folder. The launch check needs every other Fork closed.
With `-BinDir <dir>` it then copies the verified binaries to `<dir>/fork-<version>`.

`pwsh scripts/workflow.ps1` is what CI runs (`.github/workflows/gh-pages.yml`, on Windows): `build.ps1` tests the
newest release and 2.15.3, the last unpacked `Fork.exe`, into `bin/`, and `scripts/publish.ps1` renders the source
tree plus those binaries into `dist`, which is uploaded as the `gh-pages` artifact and published to GitHub Pages.

## How it works

- **Highlighting.** Fork colors diffs in its native `biturbo.dll` (`bt_highlight_syntax`). The installer renames that
  DLL to `biturbo_original.dll` and puts this crate in its place. The crate highlights `.spi` and `.spir` files with a
  tokenizer ported from the compiler's (`src/lexer.rs`, both editions, with comment, raw string and macro state carried
  across lines), and every other export is a linker forwarder to the original. `build.rs` generates the forwarders
  from the installed DLL's export table, so each Fork version gets its own list. They need `rust-lld`
  (`.cargo/config.toml`), because `link.exe` doesn't create forwarders. `target/release/verify.exe <fork-dir>` checks
  that every original export is present and that Spiral and Rust both highlight.
- **Font size.** When `Fork.exe` is a plain .NET assembly (2.15), `patcher/patch.cs` lowers the Preferences minimum
  (`CodeEditorMinFontSize`, default here 1). It keeps `Fork.exe.orig`. From 2.23 on, `Fork.exe` is packed. The patcher
  leaves it alone, and `-FontSize` writes the size to `settings.json` instead: Fork 2.23 keeps a smaller stored value,
  and only typing into the Preferences font box resets it to 10.

Fork's 9 highlight styles are fixed per theme. Keywords, symbol keywords and number suffixes use Keyword. Uppercase
names and macro type variables use Type. Unary operators use Command. Escapes and `///>` directives use Attribute.
Macro term variables use Variable. Symbols (`.name`) use Value. Numbers and booleans use Number.
