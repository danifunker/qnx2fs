# qnx2fs

Read and write **QNX 2** filesystems: QNX 2.x volumes, and the diskettes and
hard-disk partitions of the Unisys **ICON** school computer and its **LEXICON**
fileserver.

- List directories, read files, and check a volume (broken chains, cross-links,
  the free-block bitmap against what files hold, recoverable deleted files).
- Create files and directories, delete, rename, change permissions and owners.
  Writes follow what QNX 4.10.01 itself writes, and were checked with its
  `chkfsys` on the [icon-emu](https://github.com/danifunker/icon-emu) emulator.
- Convert text: QNX 2 ends lines with RS (`0x1E`) instead of LF.

It has no dependencies and works on any `Read + Seek` (and `Write` for edits)
device, including in WebAssembly.

```rust
use qnx2fs::{lf_to_rs, NewEntry, Volume};

let image = std::fs::read("diskette.img")?;
let mut vol = Volume::open(std::io::Cursor::new(image), 0)?;
for e in vol.list("/")? {
    println!("{:16} {:>8} {}", e.name, e.size, e.describe());
}
vol.mkdir("/docs", &NewEntry::default())?;
vol.put("/docs/notes", &lf_to_rs(b"hello\n"), &NewEntry::default())?;
std::fs::write("diskette.img", vol.into_inner().into_inner())?;
```

A volume inside a partitioned hard disk is opened at its byte offset. On ICON
hard disks, partition types 07, 08 and 09 hold QNX 2, and the partition's start
field is a 1-based block number: the volume starts at byte
`(start - 1) * 512`.

Dates: the ICON stores local time with a 4-bit year (1980-1995). Edits stamp the
time set with `Volume::set_clock`, or the system clock. On
`wasm32-unknown-unknown` there is no clock, so set it.

Used by [icon-emu](https://github.com/danifunker/icon-emu) and
[RustyBackup](https://github.com/danifunker/rusty-backup).

## License

MIT. See [LICENSE](LICENSE).

The format notes build on Mitchell Schoenbrun's public-domain `q2files.c` and
`q2read.c`.
