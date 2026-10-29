# Changelog

All notable changes to this crate are documented here. This project follows
[Semantic Versioning](https://semver.org/).

## 0.1.0

First release. Extracted from RustyBackup's QNX 2 support (same author) and
relicensed MIT, so that MIT projects such as icon-emu can use it.

- Read: list directories (deleted entries on request), read files, look up paths.
- Write, as QNX 4.10.01 itself writes (checked with its `chkfsys` in icon-emu):
  create files and directories, delete, rename, set permissions and owners.
- Path-based helpers: `put`, `get`, `mkdir`, `remove`, `rename_path`, `list`.
- `Volume::check`: broken chains, cross-links, bitmap against the blocks files
  hold, recoverable deleted files.
- Text: `rs_to_lf`, `lf_to_rs`, `looks_like_rs_text`. Names: `validate_name`,
  `sanitize_name`. Dates: `decode_date`, `encode_date`, `format_date`.
- No dependencies; Rust 1.73; builds for `wasm32-unknown-unknown` (set the clock
  with `Volume::set_clock` there).
