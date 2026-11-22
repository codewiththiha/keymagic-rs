//! Update checks against a mocked `updates.json` in the KM3 schema.

use keymagic_host::updater::{Manifest, fetch, postponed};
use std::error::Error;
use std::io::{self, Read, Write};

type TestResult = Result<(), Box<dyn Error>>;

const MOCK: &str = r#"{
  "name": "KeyMagic",
  "platforms": {
    "windows": {
      "x86_64": {
        "version": "0.4.0",
        "url": "https://example.test/KeyMagic3-Setup-0.4.0-x64.exe",
        "sha256": "aa11"
      }
    },
    "linux": {
      "x86_64": {
        "version": "0.3.0",
        "packages": {
          "deb": { "url": "https://example.test/keymagic3.deb", "sha256": "bb22" }
        }
      }
    }
  }
}"#;

#[test]
fn newer_windows_release_is_offered_with_its_checksum() -> TestResult {
    let manifest = Manifest::parse(MOCK.as_bytes())?;
    let update = manifest
        .newer_than("windows", "x86_64", None, "0.3.0")?
        .ok_or("expected an update")?;
    assert_eq!(update.version, "0.4.0");
    assert_eq!(
        update.url,
        "https://example.test/KeyMagic3-Setup-0.4.0-x64.exe"
    );
    assert_eq!(update.sha256, "aa11");
    Ok(())
}

#[test]
fn same_or_older_release_is_not_offered() -> TestResult {
    let manifest = Manifest::parse(MOCK.as_bytes())?;
    assert_eq!(
        manifest.newer_than("windows", "x86_64", None, "0.4.0")?,
        None
    );
    assert_eq!(
        manifest.newer_than("windows", "x86_64", None, "0.10.0")?,
        None
    );
    Ok(())
}

#[test]
fn semver_compares_numbers_not_text() -> TestResult {
    let json = MOCK.replace("0.4.0", "0.10.0");
    let manifest = Manifest::parse(json.as_bytes())?;
    assert!(
        manifest
            .newer_than("windows", "x86_64", None, "0.9.0")?
            .is_some()
    );
    Ok(())
}

#[test]
fn linux_offers_the_requested_package() -> TestResult {
    let manifest = Manifest::parse(MOCK.as_bytes())?;
    let update = manifest
        .newer_than("linux", "x86_64", Some("deb"), "0.2.9")?
        .ok_or("expected a deb update")?;
    assert_eq!(update.url, "https://example.test/keymagic3.deb");
    assert_eq!(update.sha256, "bb22");
    assert_eq!(
        manifest.newer_than("linux", "x86_64", Some("rpm"), "0.2.9")?,
        None
    );
    Ok(())
}

#[test]
fn unknown_platform_or_missing_checksum_is_handled() -> TestResult {
    let manifest = Manifest::parse(MOCK.as_bytes())?;
    assert_eq!(
        manifest.newer_than("macos", "aarch64", None, "0.1.0")?,
        None
    );
    let no_checksum = MOCK.replace("\"sha256\": \"aa11\"", "\"sha256\": null");
    let update = Manifest::parse(no_checksum.as_bytes())?
        .newer_than("windows", "x86_64", None, "0.3.0")?
        .ok_or("expected an update")?;
    assert_eq!(update.sha256, "");
    Ok(())
}

#[test]
fn malformed_versions_and_manifests_are_errors() -> TestResult {
    let manifest = Manifest::parse(MOCK.as_bytes())?;
    assert!(
        manifest
            .newer_than("windows", "x86_64", None, "not-a-version")
            .is_err()
    );
    assert!(Manifest::parse(b"{\"platforms\": 3}").is_err());
    Ok(())
}

#[test]
fn remind_after_postpones_until_its_date() -> TestResult {
    assert!(postponed("2026-11-22", Some("2026-11-23"))?);
    assert!(!postponed("2026-11-23", Some("2026-11-23"))?);
    assert!(!postponed("2026-11-22", None)?);
    assert!(postponed("2026-11-22", Some("2026/11/23")).is_err());
    assert!(postponed("22-11-2026", None).is_err());
    Ok(())
}

#[test]
fn fetch_downloads_and_parses_the_manifest_over_http() -> TestResult {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}/updates.json", listener.local_addr()?);
    let server = std::thread::spawn(move || -> io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request)?;
        write!(
            stream,
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: application/json\r\n",
                "Content-Length: {}\r\n",
                "Connection: close\r\n\r\n{}"
            ),
            MOCK.len(),
            MOCK
        )
    });
    let manifest = fetch(&url)?;
    server
        .join()
        .map_err(|_| io::Error::other("server thread panicked"))??;
    assert_eq!(manifest, Manifest::parse(MOCK.as_bytes())?);
    Ok(())
}
