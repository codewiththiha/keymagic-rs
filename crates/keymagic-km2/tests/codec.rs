//! Exercise the codec with corpus and explicitly constructed format cases.

use keymagic_km2::{Km2ErrorKind, parse, parse_document, read_info_only, write, write_document};
use keymagic_types::{Keyboard, OutputElement, PatternElement, Rule, VirtualKey};
use std::{error::Error, fs, path::Path};

fn corpus(path: &Path, checked: &mut usize) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            corpus(&path, checked)?;
        } else if path.extension().is_some_and(|extension| extension == "km2") {
            let bytes = fs::read(&path)?;
            let document = parse_document(&bytes)?;
            let encoded = write_document(&document)?;
            let mismatch = bytes.iter().zip(&encoded).position(|(a, b)| a != b);
            assert_eq!(bytes.len(), encoded.len(), "{} length", path.display());
            assert_eq!(
                mismatch,
                None,
                "{} mismatch at {mismatch:?}",
                path.display()
            );
            assert_eq!(parse(&write(&document.keyboard)?)?, document.keyboard);
            assert_eq!(read_info_only(&bytes)?, document.keyboard.info);
            *checked += 1;
        }
    }
    Ok(())
}

#[test]
fn corpus_round_trips_byte_identically() -> Result<(), Box<dyn Error>> {
    let mut checked = 0;
    corpus(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/km2")),
        &mut checked,
    )?;
    assert_eq!(checked, 5);
    Ok(())
}

#[test]
fn truncated_offsets() {
    for (bytes, offset) in [
        (&b"KM"[..], 0),
        (&b"KMKL\x01\x05\x00"[..], 6),
        (&b"KMKL\x01\x05\x00\x00\x00\x00\x00\x00\x01"[..], 13),
    ] {
        let err = parse(bytes).err();
        assert_eq!(
            err.map(|e| (e.kind, e.offset)),
            Some((Km2ErrorKind::Truncated, offset))
        );
    }
}

#[test]
fn headers_for_all_versions() -> Result<(), Box<dyn Error>> {
    for (minor, header) in [
        (
            3,
            vec![b'K', b'M', b'K', b'L', 1, 3, 0, 0, 0, 0, 1, 0, 0, 0],
        ),
        (
            4,
            vec![b'K', b'M', b'K', b'L', 1, 4, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
        ),
        (
            5,
            vec![
                b'K', b'M', b'K', b'L', 1, 5, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0,
            ],
        ),
    ] {
        let doc = parse_document(&header)?;
        assert_eq!(doc.minor_version, minor);
        assert!(doc.keyboard.options.right_alt);
        assert_eq!(write_document(&doc)?, header);
        assert_eq!(write(&doc.keyboard)?[5], 5);
    }
    Ok(())
}

#[test]
fn contextual_opcodes_and_index_boundaries() -> Result<(), Box<dyn Error>> {
    let kb = Keyboard {
        strings: vec!["a😀".into(), "x".into()],
        rules: vec![Rule {
            lhs: vec![
                PatternElement::AnyOf(0),
                PatternElement::NotAnyOf(1),
                PatternElement::VirtualKeys(vec![VirtualKey::Shift, VirtualKey::KeyA]),
            ],
            rhs: vec![
                OutputElement::VarIndexed { var: 1, by_ref: 0 },
                OutputElement::Ref(1),
                OutputElement::Null,
                OutputElement::State(3),
            ],
            source: None,
        }],
        ..Keyboard::default()
    };
    assert_eq!(parse(&write(&kb)?)?, kb);
    Ok(())
}

#[test]
fn invalid_utf16_and_opcode_are_errors() {
    let mut invalid =
        b"KMKL\x01\x05\x01\x00\x00\x00\x00\x00\x01\x00\x00\x00\x01\x00\x01\x00\x00\xD8".to_vec();
    assert_eq!(
        parse(&invalid).err().map(|e| e.kind),
        Some(Km2ErrorKind::InvalidUtf16)
    );
    invalid =
        b"KMKL\x01\x05\x00\x00\x00\x00\x01\x00\x01\x00\x00\x00\x01\x00\x01\x00\xFF\x00\x00\x00"
            .to_vec();
    assert_eq!(
        parse(&invalid).err().map(|e| e.kind),
        Some(Km2ErrorKind::InvalidOpcode)
    );
}

#[test]
fn info_only_does_not_require_rule_bytes() -> Result<(), Box<dyn Error>> {
    let bytes = b"KMKL\x01\x05\x00\x00\x00\x00\x01\x00\x01\x00\x00\x00\x01\x00";
    assert!(read_info_only(bytes)?.entries.is_empty());
    assert_eq!(
        parse(bytes).err().map(|e| e.kind),
        Some(Km2ErrorKind::Truncated)
    );
    Ok(())
}

#[test]
fn invalid_boundaries_and_values_are_rejected() {
    let cases: &[(&[u8], Km2ErrorKind, usize)] = &[
        (b"NOPE\x01\x05", Km2ErrorKind::InvalidMagic, 0),
        (b"KMKL\x01\x06", Km2ErrorKind::UnsupportedVersion, 4),
        (
            b"KMKL\x01\x05\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x01\x00",
            Km2ErrorKind::InvalidBoolean,
            12,
        ),
        (
            b"KMKL\x01\x05\x00\x00\x00\x00\x01\x00\x01\x00\x00\x00\x01\x00\x01\x00\xF1\x00\x00\x00",
            Km2ErrorKind::Truncated,
            22,
        ),
        (
            b"KMKL\x01\x05\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x01\x00\xAA",
            Km2ErrorKind::TrailingBytes,
            18,
        ),
    ];
    for (bytes, kind, offset) in cases {
        assert_eq!(
            parse(bytes).err().map(|e| (e.kind, e.offset)),
            Some((*kind, *offset))
        );
    }
}

#[test]
#[ignore = "requires binaries from the pinned upstream compiler"]
fn matches_upstream_writer() -> Result<(), Box<dyn Error>> {
    let directory = std::env::var("KEYMAGIC_REFERENCE_KM2_DIRECTORY")?;
    let mut count = 0;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let bytes = fs::read(&path)?;
        let encoded = write(&parse(&bytes)?)?;
        let mismatch = bytes.iter().zip(&encoded).position(|(a, b)| a != b);
        assert_eq!(bytes.len(), encoded.len(), "{} length", path.display());
        assert_eq!(
            mismatch,
            None,
            "{} mismatch at {mismatch:?}",
            path.display()
        );
        count += 1;
    }
    assert!(count >= 30);
    Ok(())
}

#[test]
fn refuses_unrepresentable_lengths_and_legacy_options() {
    use keymagic_km2::{Document, Km2ErrorKind};
    use keymagic_types::{InfoEntry, KeyboardInfo, LayoutOptions};
    let mut kb = Keyboard {
        strings: vec!["a".repeat(65_536)],
        ..Keyboard::default()
    };
    assert_eq!(
        write(&kb).err().map(|error| error.kind),
        Some(Km2ErrorKind::TooLarge)
    );
    kb.strings.clear();
    kb.info = KeyboardInfo {
        entries: vec![InfoEntry {
            id: *b"test",
            data: vec![0; 65_536],
        }],
    };
    assert_eq!(
        write(&kb).err().map(|error| error.kind),
        Some(Km2ErrorKind::TooLarge)
    );
    let doc = Document {
        keyboard: kb,
        minor_version: 3,
        padding: 0,
    };
    assert_eq!(
        write_document(&doc).err().map(|error| error.kind),
        Some(Km2ErrorKind::InvalidKeyboard)
    );
    let doc = Document {
        keyboard: Keyboard {
            options: LayoutOptions {
                right_alt: false,
                ..LayoutOptions::default()
            },
            ..Keyboard::default()
        },
        minor_version: 4,
        padding: 0,
    };
    assert_eq!(
        write_document(&doc).err().map(|error| error.kind),
        Some(Km2ErrorKind::InvalidKeyboard)
    );
}
