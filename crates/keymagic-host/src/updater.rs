//! Reads the `updates.json` manifest and reports a newer release.

use crate::invalid_data;
use semver::Version;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io;

/// Release manifest: platform, then architecture, then the release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    platforms: BTreeMap<String, BTreeMap<String, Target>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Target {
    version: String,
    url: Option<String>,
    sha256: Option<String>,
    packages: Option<BTreeMap<String, Package>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Package {
    url: String,
    sha256: Option<String>,
}

/// A release newer than the installed version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub version: String,
    pub url: String,
    /// Empty when the manifest gives no checksum.
    pub sha256: String,
}

impl Manifest {
    /// Reads the manifest JSON.
    #[doc = "# Errors\n\nReturns an error when the JSON has the wrong shape."]
    pub fn parse(json: &[u8]) -> io::Result<Self> {
        serde_json::from_slice(json).map_err(invalid_data)
    }

    /// Returns the release for `platform` and `arch` if newer than `current`.
    #[doc = "# Errors\n\nReturns an error for a malformed version string."]
    pub fn newer_than(
        &self,
        platform: &str,
        arch: &str,
        package: Option<&str>,
        current: &str,
    ) -> io::Result<Option<Update>> {
        let Some(target) = self.target(platform, arch) else {
            return Ok(None);
        };
        let latest = Version::parse(&target.version).map_err(invalid_data)?;
        if latest <= Version::parse(current).map_err(invalid_data)? {
            return Ok(None);
        }
        let source = package.map_or_else(
            || target.url.clone().map(|url| (url, target.sha256.clone())),
            |name| {
                target
                    .packages
                    .as_ref()
                    .and_then(|packages| packages.get(name))
                    .map(|found| (found.url.clone(), found.sha256.clone()))
            },
        );
        Ok(source.map(|(url, sha256)| Update {
            version: target.version.clone(),
            url,
            sha256: sha256.unwrap_or_default(),
        }))
    }

    fn target(&self, platform: &str, arch: &str) -> Option<&Target> {
        self.platforms.get(platform)?.get(arch)
    }
}

/// True while `today` is before `remind_after`. Dates are `YYYY-MM-DD`.
#[doc = "# Errors\n\nReturns an error when a date is not `YYYY-MM-DD`."]
pub fn postponed(today: &str, remind_after: Option<&str>) -> io::Result<bool> {
    check_date(today)?;
    remind_after.map_or(Ok(false), |remind| {
        check_date(remind)?;
        Ok(today < remind)
    })
}

fn check_date(text: &str) -> io::Result<()> {
    let well_formed = text.len() == 10
        && text.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        });
    if well_formed {
        Ok(())
    } else {
        Err(invalid_data(format!("expected YYYY-MM-DD, got {text}")))
    }
}
