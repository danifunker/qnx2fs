use super::*;
use std::io::Cursor;

/// A 16-block volume: root at block 3 holding `HELLO` (block 5, 6 bytes) and dir `sub` (block 4).
fn sample() -> Vec<u8> {
    let mut img = vec![0u8; 16 * 512];
    let entry = |img: &mut Vec<u8>, at: usize, name: &[u8], first: u32, attr: u8| {
        img[at] = USED;
        img[at + 1..at + 5].copy_from_slice(&first.to_le_bytes());
        img[at + 26] = attr;
        img[at + 24] = 0x21;
        img[at + 25] = 0x01;
        img[at + 31..at + 31 + name.len()].copy_from_slice(name);
    };
    entry(&mut img, ROOT_AT, b"", 3, 0x25);
    let hdr = |img: &mut Vec<u8>, block: usize, size: u32| {
        let at = (block - 1) * 512;
        img[at + 8..at + 12].copy_from_slice(&size.to_le_bytes());
    };
    hdr(&mut img, 3, 4 + 2 * ENTRY as u32);
    let dir = 2 * 512 + 16 + 4;
    entry(&mut img, dir, b"HELLO", 5, 0x01);
    entry(&mut img, dir + ENTRY, b"sub", 4, 0x25);
    hdr(&mut img, 4, 4);
    hdr(&mut img, 5, 6);
    img[4 * 512 + 16..4 * 512 + 22].copy_from_slice(b"hello\n");
    img
}

/// `sample()` plus a deleted `GONE` (block 6) in the root and a bitmap file at block 2.
fn sample_with_bitmap() -> Vec<u8> {
    let mut img = sample();
    let dir = 2 * 512 + 16 + 4;
    let at = dir + 2 * ENTRY;
    img[at + 1..at + 5].copy_from_slice(&6u32.to_le_bytes());
    img[at + 26] = 0x11;
    img[at + 31..at + 35].copy_from_slice(b"GONE");
    let at = dir + 3 * ENTRY;
    img[at] = USED;
    img[at + 1..at + 5].copy_from_slice(&2u32.to_le_bytes());
    img[at + 26] = 0x01;
    img[at + 31..at + 37].copy_from_slice(b"bitmap");
    img[2 * 512 + 8..2 * 512 + 12].copy_from_slice(&(4 + 4 * ENTRY as u32).to_le_bytes());
    img[512 + 8..512 + 12].copy_from_slice(&2u32.to_le_bytes());
    // Blocks 1-5 in use (bits 0-4); 6 (the deleted file) free.
    img[512 + 16] = 0x1F;
    img[5 * 512 + 8..5 * 512 + 12].copy_from_slice(&3u32.to_le_bytes());
    img[5 * 512 + 16..5 * 512 + 19].copy_from_slice(b"old");
    img
}

/// `sample_with_bitmap()` with the root's header naming its entry (block 1, slot 1) as QNX writes it.
fn editable_sample() -> Cursor<Vec<u8>> {
    let mut img = sample_with_bitmap();
    img[2 * 512 + 16..2 * 512 + 20].copy_from_slice(&[1, 0, 1, 0]);
    Cursor::new(img)
}

/// A 640K (1280-block) diskette exactly as the ICON's `dinit 2` leaves it.
fn blank_640k() -> Cursor<Vec<u8>> {
    let head = include_bytes!("../tests/fixtures/blank640-head.bin");
    let mut img = vec![0u8; 655_360];
    img[..head.len()].copy_from_slice(head);
    Cursor::new(img)
}

#[test]
fn detects_and_lists_a_volume() {
    let img = sample();
    assert!(looks_like_qnx2(&mut Cursor::new(img.clone()), 0));
    let mut vol = Volume::open(Cursor::new(img), 0).expect("open");
    let root = vol.root();
    let list = vol.list_dir(&root, false).unwrap();
    let names: Vec<_> = list.iter().map(|e| (e.name.as_str(), e.is_dir)).collect();
    assert_eq!(names, [("HELLO", false), ("sub", true)]);
    assert_eq!(list[0].size, 6);
    assert_eq!(list[0].path, "/HELLO");
    assert_eq!(vol.read_file(&list[0], usize::MAX).unwrap(), b"hello\n");
    assert!(vol.list_dir(&list[1], false).unwrap().is_empty());
}

#[test]
fn rejects_fat_and_blank_disks() {
    assert!(!looks_like_qnx2(&mut Cursor::new(vec![0u8; 8192]), 0));
    let mut fat = vec![0u8; 8192];
    fat[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
    assert!(!looks_like_qnx2(&mut Cursor::new(fat), 0));
    assert!(matches!(
        Volume::open(Cursor::new(vec![0u8; 8192]), 0),
        Err(Error::NotQnx2)
    ));
}

#[test]
fn a_looping_chain_is_an_error_not_a_hang() {
    let mut img = sample();
    img[4 * 512 + 4..4 * 512 + 8].copy_from_slice(&5u32.to_le_bytes());
    let mut vol = Volume::open(Cursor::new(img), 0).unwrap();
    assert!(vol.read_chain(5, usize::MAX).is_err());
}

#[test]
fn icon_dates_match_what_qnx_files_prints() {
    let show = |w0, w1| format_date(decode_date(w0, w1).unwrap());
    // Each pair as QNX 4.10.01's `files` lists it in icon-emu.
    assert_eq!(show(0x5539, 0x2A91), "1985-05-28 15:01:37"); // 28-May-85 3:01pm
    assert_eq!(show(0xBA3A, 0x0642), "1991-10-29 00:26:42"); // 29-Oct-91 12:26am
    assert_eq!(show(0x832F, 0x3976), "1988-03-23 16:05:10"); // 23-Mar-88 4:05pm
    assert_eq!(decode_date(0, 0), None);
    // A real ST-251 installed with the clock unset: QNX lists `0-----80 12:12am`.
    assert_eq!(show(0x0000, 0x02D0), "1980-01-01 00:12:00");
    assert_eq!(decode_date(0x0D00, 0), None, "month 13 is not a date");
    assert_eq!(decode_date(0x823C, 0), None, "30 February is not a date");
    for t in [
        decode_date(0x5539, 0x2A91).unwrap(),
        decode_date(0xBA3A, 0x0642).unwrap(),
    ] {
        let [w0, w1] = encode_date(t);
        assert_eq!(decode_date(w0, w1), Some(t));
    }
    // The 4-bit year stops at 1995.
    assert_eq!(encode_date(2_000_000_000)[0] >> 12, 15);
    assert_eq!(format_date(0), "1970-01-01 00:00:00");
    assert_eq!(format_date(951_782_400), "2000-02-29 00:00:00");
}

#[test]
fn owner_permissions_and_attributes_are_decoded() {
    let mut vol = Volume::open(Cursor::new(sample()), 0).unwrap();
    let list = vol.list("/").unwrap();
    assert_eq!(list[0].owner_label(), "0.0");
    assert_eq!(list[0].unix_mode(), 0o100444);
    assert_eq!(list[0].describe(), "owner r---- group r---- other r----");
    assert_eq!(list[1].unix_mode(), 0o040755);
    assert_eq!(letters(0x35, true), "rc-m");
    assert_eq!(letters(0x19, false), "r--em");
    assert_eq!(unix_class(0x19, false), 0o5);
}

#[test]
fn deleted_entries_list_only_when_asked_and_read_back() {
    let mut vol = Volume::open(Cursor::new(sample_with_bitmap()), 0).unwrap();
    let root = vol.root();
    assert!(!vol
        .list_dir(&root, false)
        .unwrap()
        .iter()
        .any(|e| e.name == "GONE"));
    let list = vol.list_dir(&root, true).unwrap();
    let gone = list
        .iter()
        .find(|e| e.name == "GONE")
        .expect("deleted entry listed");
    assert_eq!(gone.deleted, Some(DeletedState::Recoverable));
    assert_eq!(vol.read_file(gone, usize::MAX).unwrap(), b"old");
    assert!(gone.describe().ends_with("deleted, recoverable"));
}

#[test]
fn check_reports_orphans_and_unmarked_blocks() {
    let img = sample_with_bitmap();
    let mut vol = Volume::open(Cursor::new(img.clone()), 0).unwrap();
    let r = vol.check().unwrap();
    assert!(r.is_clean(), "{:?}", r.errors);
    assert!(r.warnings.iter().all(|w| w.debug), "{:?}", r.warnings);
    // Mark block 8 used (orphan) and free block 5 (HELLO's).
    let mut bad = img;
    bad[512 + 16] = 0x8F;
    let mut vol = Volume::open(Cursor::new(bad), 0).unwrap();
    let r = vol.check().unwrap();
    assert!(r.errors.iter().any(|e| e.code == "UnmarkedBlocks"));
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "OrphanedBlocks" && w.message.ends_with(": 8")));
}

#[test]
fn text_converts_both_ways_and_binaries_are_left_alone() {
    assert!(looks_like_rs_text(b"line one\x1eline two\x1e"));
    assert_eq!(rs_to_lf(b"a\x1eb\x1e"), b"a\nb\n");
    assert_eq!(lf_to_rs(b"a\nb\r\nc"), b"a\x1eb\x1ec");
    assert!(!looks_like_rs_text(b"\x01\x84\x9f\x00\x1e"));
    assert!(!looks_like_rs_text(b"plain unix text\n"));
}

#[test]
fn edits_round_trip_and_check_clean() {
    let mut vol = Volume::open(editable_sample(), 0).unwrap();
    let root = vol.root();
    let body: Vec<u8> = (0..1500u32).map(|i| i as u8).collect();
    let f = vol
        .create_file(
            &root,
            "data.bin",
            &mut &body[..],
            body.len() as u64,
            &NewEntry::default(),
        )
        .unwrap();
    assert_eq!(vol.read_file(&f, usize::MAX).unwrap(), body);
    let d = vol
        .create_dir(&root, "newdir", &NewEntry::default())
        .unwrap();
    let inner = vol
        .create_file(&d, "x", &mut &b"hi\x1e"[..], 3, &NewEntry::default())
        .unwrap();
    assert_eq!(vol.read_file(&inner, usize::MAX).unwrap(), b"hi\x1e");
    // The new directory's header points back at its slot in the root.
    let loc = vol.self_loc(d.first).unwrap();
    assert_eq!(loc.dir, vol.root_first);
    let r = vol.check().unwrap();
    assert!(r.is_clean(), "{:?}", r.errors);
    assert!(r.warnings.iter().all(|w| w.debug), "{:?}", r.warnings);

    vol.rename(&root, &f, "renamed").unwrap();
    let names: Vec<String> = vol
        .list_dir(&root, false)
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert!(names.contains(&"renamed".to_string()) && !names.contains(&"data.bin".to_string()));
    let free_before = vol.free_blocks();
    let renamed = vol.lookup("/renamed").unwrap();
    vol.delete(&root, &renamed).unwrap();
    assert!(vol.free_blocks() > free_before);
    assert!(
        vol.delete(&root, &d).is_err(),
        "a non-empty directory stays"
    );
    vol.delete(&d, &inner).unwrap();
    vol.delete(&root, &d).unwrap();
    let r = vol.check().unwrap();
    assert!(r.is_clean(), "{:?}", r.errors);
}

#[test]
fn names_and_extent_limits_follow_qnx() {
    assert!(validate_name("ok_name.1$").is_ok());
    for bad in ["", "$lead", "has space", "seventeen_chars_x", "slash/"] {
        assert!(validate_name(bad).is_err(), "{bad}");
    }
    // 65520 data bytes fill one 128-block extent; one more byte needs a second extent.
    assert_eq!(blocks_for(65520), 128);
    assert_eq!(blocks_for(65521), 129);
    assert_eq!(blocks_for(0), 1);
    assert_eq!(qnx_bits(0o644, false), (0x17, 0x01, 0x01));
    assert_eq!(qnx_bits(0o755, true), (0x35, 0x01, 0x01));
}

#[test]
fn sanitized_names_are_valid() {
    assert_eq!(sanitize_name("My Story.txt"), "My_Story.txt");
    assert_eq!(sanitize_name("$secret"), "secret");
    assert_eq!(
        sanitize_name("a_very_long_file_name.txt"),
        "a_very_long_.txt"
    );
    assert_eq!(sanitize_name("nodotbutverylongname"), "nodotbutverylong");
    assert_eq!(sanitize_name("é"), "_");
    assert_eq!(sanitize_name(""), "file");
    for n in [
        "My Story.txt",
        "$secret",
        "a_very_long_file_name.txt",
        "x/y",
        "é",
    ] {
        assert!(validate_name(&sanitize_name(n)).is_ok(), "{n}");
    }
}

#[test]
fn a_dinit_diskette_takes_files_and_folders_by_path() {
    let mut vol = Volume::open(blank_640k(), 0).unwrap();
    vol.set_clock(decode_date(0xBA3A, 0x0642).unwrap());
    assert_eq!(vol.total_blocks(), 1280);
    let names: Vec<String> = vol.list("/").unwrap().into_iter().map(|e| e.name).collect();
    assert_eq!(names, ["bitmap"]);
    let free = vol.free_blocks();

    vol.mkdir("/docs", &NewEntry::default()).unwrap();
    let text = lf_to_rs(b"first line\nsecond line\n");
    let e = vol.put("/docs/notes", &text, &NewEntry::default()).unwrap();
    assert_eq!(e.path, "/docs/notes");
    assert_eq!(e.modified(), decode_date(0xBA3A, 0x0642));
    // Larger than one extent: 70,000 bytes take two.
    let big: Vec<u8> = (0..70_000u32).map(|i| (i * 7) as u8).collect();
    vol.put("/big.bin", &big, &NewEntry::default()).unwrap();
    // Enough files to grow the folder past its first ten slots.
    for i in 0..12 {
        vol.put(&format!("/docs/f{i}"), b"x", &NewEntry::default())
            .unwrap();
    }
    assert!(matches!(
        vol.put("/big.bin", b"again", &NewEntry::default()),
        Err(Error::AlreadyExists(_))
    ));
    assert!(matches!(vol.get("/nope"), Err(Error::NotFound(_))));

    // Read it all back from a fresh open of the same bytes.
    let img = vol.into_inner();
    let mut vol = Volume::open(img, 0).unwrap();
    assert_eq!(
        rs_to_lf(&vol.get("/docs/notes").unwrap()),
        b"first line\nsecond line\n"
    );
    assert_eq!(vol.get("/big.bin").unwrap(), big);
    assert_eq!(vol.list("/docs").unwrap().len(), 13);
    let r = vol.check().unwrap();
    assert!(r.is_clean(), "{:?}", r.errors);
    assert!(r.warnings.iter().all(|w| w.debug), "{:?}", r.warnings);

    vol.rename_path("/big.bin", "big2.bin").unwrap();
    vol.remove("/big2.bin").unwrap();
    assert!(vol.remove("/docs").is_err(), "a folder with files stays");
    for i in 0..12 {
        vol.remove(&format!("/docs/f{i}")).unwrap();
    }
    vol.remove("/docs/notes").unwrap();
    vol.remove("/docs").unwrap();
    assert_eq!(vol.free_blocks(), free);
    assert!(vol.check().unwrap().is_clean());
}
