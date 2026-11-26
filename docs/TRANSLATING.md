# Translating RCleaner

English · [Español](TRANSLATING.es.md)

Every text in RCleaner (the app, the cleaner names and descriptions, error
messages and the `rcl` command line) lives in one JSON file per language in
[`locales/`](../locales). The app and the Rust engine read the same files, so a
language added there shows up everywhere. You don't need to know Rust or Svelte.

Available languages: **English** (`en.json`, the reference) and **Spanish**
(`es.json`).

## Add a new language

1. Copy `locales/en.json` to `locales/<code>.json`, using the
   [language code](https://en.wikipedia.org/wiki/List_of_ISO_639_language_codes):
   `fr.json`, `pt.json`, `de.json`… Use a region only when it really differs:
   `pt-BR.json`.
2. Change `_meta.name` to the language's own name: `"Français"`, `"Português"`.
3. Translate the **values**. Never change the **keys** (the left side).
4. Check it (see below) and open a pull request named `i18n(<code>): …`.

That's it: there's no code to touch. The app lists every file in `locales/` in
*Settings → Language* and picks it automatically when it matches the Windows
language.

You don't have to finish everything at once. Any key you leave out falls back to
English, so a partial translation is still useful. Delete the lines you haven't
translated rather than leaving English text in your file; that way the missing
parts are easy to spot later.

## Rules for values

- **Keep the `{variables}`** exactly as they are: `"{size} free"` →
  `"{size} libres"`. You can move them around in the sentence but not rename,
  translate or remove them.
- **Plurals** use two keys: `_one` for exactly 1 and `_other` for everything else.

  ```json
  "files_one": "{n} file",
  "files_other": "{n} files"
  ```

  If your language doesn't change the word, write the same text in both.
- Keep **brand and technical names** as they are: Windows, Steam, `node_modules`,
  `%TEMP%`, `pip cache purge`, file paths and command examples in backticks.
- Write in the **tone of the app**: short, plain and friendly; address the user
  the way apps normally do in your language.
- Use the typographic characters of your language (quotes, ellipsis `…`).
- Keep the JSON valid: double quotes, commas between entries, and `\\` for a
  backslash (`"C:\\Windows\\Temp"`).

## How keys are organized

| Section | What it covers |
|---|---|
| `ui.*` | Everything you see in the app windows |
| `module.*` | Cleaner names (`name`), descriptions (`desc`) and the rows they produce (`target`, `detail`) |
| `mode.*` | Names and descriptions of the built-in modes |
| `category.*`, `risk.*` | Group names and risk levels |
| `error.*`, `app.*`, `core.*` | Error and warning messages |
| `cli.*` | Output of the `rcl` command line |

## Check your translation

```powershell
cargo test -p crp-core i18n
```

The test fails if your file has invalid JSON, a key that doesn't exist in
`en.json`, or a `{variable}` that differs from English. Then run the app
(`cd app; cargo tauri dev`), choose your language in *Settings → Language* and
look for text that is cut off or doesn't fit.

If you can't build the project, open the pull request anyway: CI runs the same
check.

## Keeping translations up to date

When someone adds a feature, they add its keys to `en.json` (and to `es.json`).
Other languages show the English text for those keys until a translator adds
them. Issues labeled `i18n` list what's missing.
