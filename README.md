<div align="center">

# 🐱 Spelly

**Type however you type. Spelly quietly fixes it.**

Forced autocorrect that gets out of your way. Backspace puts back what you wrote.

<sub>Powered by <a href="https://github.com/Automattic/harper">Harper</a></sub>

</div>

---

## What it does

Finish a word and Spelly fixes it on the spot: spelling, spaces, punctuation, capitals, and the words that sound alike (*their / they're*, *then / than*). No suggestion popups. If it was right the first time, press **Backspace** and your original comes back, and Spelly leaves that word alone from then on, like a phone keyboard.

Anything it isn't sure about gets a highlight instead of an edit.

## Dictionaries

Plain text, one file per pack in [`dictionaries/`](dictionaries/). Easy for a person or an agent to extend.

```text
@kind proper         # names and titles (default); use `common` for ordinary words
Naruto               # a known word, canonical casing
narto -> Naruto      # forced correction
```

Packs: [`anime.txt`](dictionaries/anime.txt), [`indian-names.txt`](dictionaries/indian-names.txt).

## Layout

| Path | What |
| :--- | :--- |
| `crates/spelly-core` | The engine: packs, forced fixes, undo. No I/O, runs anywhere |
| `harper/` | Our Harper fork: dictionary, grammar rules, context ranking |
| `dictionaries/` | The word packs |
| `crates/spelly-win` | The Windows app: keyboard hook, tray icon, key injection |
| `crates/spelly-mac` | The macOS app: event tap, menu bar item, key injection |
| `packaging/` | Windows installer script, macOS bundle script, icon generator |

## Install

| | |
| :--- | :--- |
| **Windows** | Run `Spelly-Setup.exe`. No admin needed. Tick "Start Spelly when I sign in" if you want it always on. |
| **macOS** | Open `Spelly-macOS.dmg`, drag Spelly to Applications, launch it, and allow it under **System Settings → Privacy & Security → Accessibility**. |

Both live in the tray / menu bar: **Pause**, **Open dictionaries folder**, **Reload dictionaries**, **Quit**.

> [!NOTE]
> Builds are unsigned. If macOS says Spelly is damaged, run `xattr -cr /Applications/Spelly.app`. On Windows, SmartScreen may ask once: **More info → Run anyway**.

## Where the files live

"Open dictionaries folder" creates the folder with the starter packs inside. Edit the `.txt` files (or let an agent), then **Reload dictionaries**.

- Windows: `%APPDATA%\Spelly\dictionaries`
- macOS: `~/Library/Application Support/Spelly/dictionaries`

It never touches text you didn't just type. Clicking, arrow keys, shortcuts and switching windows all make it forget the line, and password boxes are skipped.

## Known limits

- Fixes reach back three words, and context fixes need the next word: `their going` is caught once you type `to `.
- An unknown capitalized word mid-sentence is assumed to be a name and only highlighted.
- English only, via Harper.
- The Windows and macOS shells are built by CI but have not yet been tried on real machines. Apps running as administrator can't be typed into by a normal-user Spelly.
- macOS skips password fields automatically (the OS hides them from Spelly); Windows checks for password boxes.
- Linux and mobile are not supported yet.

Dev: `cargo test`. Needs current stable Rust.

## License

Apache-2.0. See [NOTICE.md](NOTICE.md).
