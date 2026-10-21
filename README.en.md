# CarambaClone

A macOS menu bar utility: a smart automatic keyboard-layout switcher for people
who type in both Russian and English. It notices a word typed in the wrong
layout, fixes it on the fly, and repairs common typos. Behavior modeled after
Punto Switcher, Caramba Switcher and YouType.

## What it does

- Automatic EN ⇄ RU correction in any app: type `ghbdtn` and get `привет`.
- Heuristics + dictionaries: obvious errors are caught instantly, the rest via
  spell checking.
- One-key everywhere-undo: ⌫, ⌥, double Shift.
- "Masking" — the signature mode: a fraction of typed Cyrillic letters is
  replaced with visually indistinguishable Latin lookalikes.
- Fully local: typed text is never stored or sent over the network.

## Screenshots

_Screenshots of the settings window, permissions wizard and menu bar icon are planned._

## Installation

1. Download the `.dmg` from Releases and drag CarambaClone into Applications.
2. There is no and will be no Apple Developer account, so the app is not
   notarized. Remove the quarantine flag:

   ```sh
   xattr -dr com.apple.quarantine /Applications/CarambaClone.app
   ```

   (alternative: System Settings → Privacy & Security → "Open Anyway").
3. Launch the app and complete the permissions wizard: "Accessibility"
   and "Input Monitoring".
4. If permissions "stick" after an update (the box is checked but the app does
   not react) — remove the entry with the "−" button and add the app again.

## Usage

The layout flag in the menu bar opens the menu (Enable / Switch layout /
Masking / Pause / stats / Settings). Reassignable hotkeys:

- Single Shift — switch layout (optional).
- Double Shift — undo the last auto-conversion.
- ⌫ within 2 s after a correction — smart undo.
- ⌥⇧R — convert the selected text.
- ⌃⌥M — toggle Masking.

## Settings

Tabs: General, Fixes, Dictionary (word exceptions), Apps (per-app exceptions,
the Games folder), Rules, Hotkeys, About.

## macOS Text Replacements

Private rule conflicts are best solved with the system option
(System Settings → Keyboard → Text Replacements) — it takes priority over
CarambaClone. Examples: `--` → `—`, `of` → `ща`, `&` → `?`, `^` → `:`.

## Troubleshooting

- App stops reacting after an update: see installation step 4.
- macOS system autocorrect interferes: disable "Correct spelling automatically".
- Terminal and some editors are excluded by default — automation is off there.

## Building from source

Requires Rust (stable), Node.js, macOS 13+ on Apple Silicon.

```sh
npm ci
cargo build -p caramba-core
cargo run -p caramba-app
# release .app/.dmg:
npm run tauri build
```

## Privacy

Typed text is never written to disk nor sent over the network. The only network
request is the weekly "Rules" data pack from GitHub; it can be disabled.

## License

MIT. Third-party assets are listed in `THIRD_PARTY_NOTICES.md`.
