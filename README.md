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

## Windows

Run `spelly.exe`. It sits in the tray (right-click: **Pause**, **Reload dictionaries**, **Quit**). Put a `dictionaries/` folder next to it and every `.txt` in there is loaded; without one, the built-in packs are used.

It never touches text you didn't just type. Clicking, arrow keys, shortcuts and switching windows all make it forget the line, and password boxes are skipped.

## Known limits

- Fixes reach back three words, and context fixes need the next word: `their going` is caught once you type `to `.
- An unknown capitalized word mid-sentence is assumed to be a name and only highlighted.
- English only, via Harper.
- Windows shell is type-checked but not yet run on a real PC. Apps running as administrator can't be typed into by a normal-user Spelly.
- Tray icon is the stock one until the cat is drawn.

Dev: `cargo test`. Needs current stable Rust.

## License

Apache-2.0. See [NOTICE.md](NOTICE.md).
