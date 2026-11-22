//! Check `KeyMagic` internal IDs and corpus names against the upstream table.

use keymagic_types::VirtualKey;
use std::{error::Error, fmt::Write, fs, path::Path};

#[test]
fn documented_names_and_ids_round_trip() -> Result<(), Box<dyn Error>> {
    let doc = include_str!("../../../docs/VIRTUAL_KEYS.md");
    let mut count = 0;
    for line in doc.lines().filter(|line| line.starts_with("| ")) {
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        let Some(id) = fields.get(1).and_then(|value| value.parse::<u16>().ok()) else {
            continue;
        };
        let name = fields[3].trim_matches('`');
        let key = VirtualKey::from_u16(id).ok_or("documented key ID")?;
        assert_eq!(key.kms_name(), name);
        assert_eq!(VirtualKey::from_kms_name(name), Some(key));
        count += 1;
    }
    assert_eq!(count, 100);
    assert_eq!(VirtualKey::from_u16(0), None);
    assert_eq!(VirtualKey::from_u16(102), None);
    assert_eq!(VirtualKey::Back as u16, 2);
    Ok(())
}

#[test]
fn documented_aliases_resolve() -> Result<(), Box<dyn Error>> {
    let doc = include_str!("../../../docs/VIRTUAL_KEYS.md");
    let table = doc
        .split("## Name aliases")
        .nth(1)
        .ok_or("aliases section")?;
    let mut count = 0;
    for line in table.lines().filter(|line| line.starts_with("| `VK_")) {
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        let id: u16 = fields[3].parse()?;
        assert_eq!(
            VirtualKey::from_kms_name(fields[1].trim_matches('`')),
            VirtualKey::from_u16(id)
        );
        count += 1;
    }
    assert_eq!(count, 19);
    assert_eq!(VirtualKey::from_kms_name("VK_NOT_A_KEY"), None);
    Ok(())
}

fn check_files(path: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            check_files(&path)?;
        } else if path.extension().is_some_and(|extension| extension == "kms") {
            let text = fs::read_to_string(&path)?;
            for line in text.lines() {
                let source = line.split("//").next().unwrap_or_default();
                for group in source
                    .split('<')
                    .skip(1)
                    .filter_map(|part| part.split('>').next())
                {
                    for name in group
                        .split('&')
                        .map(str::trim)
                        .filter(|name| name.starts_with("VK_"))
                    {
                        assert!(
                            VirtualKey::from_kms_name(name).is_some(),
                            "{}: {name}",
                            path.display()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn corpus_virtual_key_names_are_known() -> Result<(), Box<dyn Error>> {
    check_files(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../corpus/kms"
    )))
}

#[test]
fn regenerated_reference_tables_match_snapshot() -> Result<(), Box<dyn Error>> {
    let doc = include_str!("../../../docs/VIRTUAL_KEYS.md");
    let mut ids =
        String::from("| ID | KM3 Rust variant | Legacy ID name |\n| ---: | --- | --- |\n");
    let mut aliases = String::from("| Alias | Variant | ID |\n| --- | --- | ---: |\n");
    for id in 2..=101 {
        let key = VirtualKey::from_u16(id).ok_or("missing key")?;
        writeln!(ids, "| {id} | `{key:?}` | `{}` |", key.kms_name())?;
        for alias in key.aliases() {
            writeln!(aliases, "| `{alias}` | `{key:?}` | {id} |")?;
        }
    }
    assert!(doc.contains(&ids));
    assert!(doc.contains(&aliases));
    assert_eq!(VirtualKey::from_u16(1), Some(VirtualKey::Null));
    Ok(())
}
