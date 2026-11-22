//! Snapshot comparison shared by the integration tests.

use std::fs;
use std::path::Path;

/// Records a failure when the stored snapshot differs from `actual`.
pub fn compare(dir: &Path, name: &str, actual: &str, failures: &mut Vec<String>) {
    let stored = fs::read_to_string(dir.join(name)).unwrap_or_default();
    if stored != actual {
        failures.push(format!(
            "snapshot {name} differs\n<<<ACTUAL\n{actual}ACTUAL>>>"
        ));
    }
}

/// Fails the test with every recorded snapshot difference.
pub fn finish(failures: &[String]) {
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
