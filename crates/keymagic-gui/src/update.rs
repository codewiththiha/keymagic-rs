//! Checks the release manifest for a newer build, off the UI thread.

use crate::state::{Host, lock};
use keymagic_host::updater::{self, Manifest, Update};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

/// The release manifest that KM3 publishes.
pub const UPDATE_JSON_URL: &str = "https://thantthet.github.io/keymagic-3/updates.json";

/// Days a "Remind me later" choice waits.
const REMIND_DAYS: i64 = 7;

const SECONDS_PER_DAY: u64 = 86_400;

/// Where the update check stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    /// No check has run in this session.
    #[default]
    Idle,
    /// The manifest is being fetched.
    Checking,
    /// The installed version is the newest.
    Current,
    /// A newer release exists.
    Available(Update),
    /// The check or the reminder failed; the text says why.
    Failed(String),
}

/// Runs the check on a worker thread and keeps its result.
#[derive(Debug, Default)]
pub struct Checker {
    status: Arc<Mutex<Status>>,
}

impl Checker {
    /// A checker that already holds `status`, for tests and snapshots.
    #[must_use]
    pub fn with_status(status: Status) -> Self {
        Self {
            status: Arc::new(Mutex::new(status)),
        }
    }

    /// The latest status.
    #[must_use]
    pub fn status(&self) -> Status {
        lock_status(&self.status).clone()
    }

    /// Replaces the status, for errors found outside the check.
    pub fn set(&self, status: Status) {
        *lock_status(&self.status) = status;
    }

    /// Fetches the manifest on a worker thread unless a check is running.
    pub fn start(&self, host: &Arc<Mutex<Host>>) {
        {
            let mut status = lock_status(&self.status);
            if *status == Status::Checking {
                return;
            }
            *status = Status::Checking;
        }
        let status = Arc::clone(&self.status);
        let host = Arc::clone(host);
        std::thread::spawn(move || {
            let outcome = check();
            lock(&host).config_mut().general.last_update_check = Some(date(0));
            *lock_status(&status) = outcome;
        });
    }
}

fn lock_status(status: &Mutex<Status>) -> MutexGuard<'_, Status> {
    status.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Fetches the manifest and compares it with this build.
fn check() -> Status {
    updater::fetch(UPDATE_JSON_URL)
        .and_then(|manifest| {
            judge(
                &manifest,
                std::env::consts::OS,
                std::env::consts::ARCH,
                linux_package(),
                env!("CARGO_PKG_VERSION"),
            )
        })
        .unwrap_or_else(|error| Status::Failed(error.to_string()))
}

/// Turns a manifest into a status for one platform and build.
fn judge(
    manifest: &Manifest,
    platform: &str,
    arch: &str,
    package: Option<&str>,
    current: &str,
) -> io::Result<Status> {
    let found = manifest.newer_than(platform, arch, package, current)?;
    Ok(match found {
        Some(update) if update.url.starts_with("https://") => Status::Available(update),
        Some(_) => Status::Failed("The release link is not HTTPS.".to_owned()),
        None => Status::Current,
    })
}

/// The Linux package format this system installs, if one is known.
fn linux_package() -> Option<&'static str> {
    if std::env::consts::OS != "linux" {
        None
    } else if Path::new("/usr/bin/dpkg").exists() {
        Some("deb")
    } else if Path::new("/usr/bin/rpm").exists() {
        Some("rpm")
    } else {
        None
    }
}

/// True while a "Remind me later" date is still in the future.
#[must_use]
pub fn postponed_now(remind_after: Option<&str>) -> bool {
    updater::postponed(&date(0), remind_after).unwrap_or(false)
}

/// Saves a reminder date `REMIND_DAYS` from today.
#[doc = "# Errors\n\nReturns an error when the config cannot be written."]
pub fn remind_later(host: &Arc<Mutex<Host>>) -> io::Result<()> {
    let mut guard = lock(host);
    guard.config_mut().general.update_remind_after = Some(date(REMIND_DAYS));
    guard.save()
}

/// The UTC date `offset` days from today, as `YYYY-MM-DD`.
#[must_use]
pub fn date(offset: i64) -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = i64::try_from(secs / SECONDS_PER_DAY).unwrap_or(0) + offset;
    let (year, month, day) = civil(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Converts days since 1970-01-01 to a Gregorian `(year, month, day)`.
const fn civil(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::{Checker, Status, civil, date, judge};
    use keymagic_host::updater::{Manifest, Update};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn manifest(version: &str, url: &str) -> Result<Manifest, Box<dyn Error>> {
        let json = format!(
            "{{\"platforms\":{{\"windows\":{{\"x86_64\":\
             {{\"version\":\"{version}\",\"url\":\"{url}\"}}}}}}}}"
        );
        Ok(Manifest::parse(json.as_bytes())?)
    }

    fn judged(version: &str, url: &str, current: &str) -> Result<Status, Box<dyn Error>> {
        Ok(judge(
            &manifest(version, url)?,
            "windows",
            "x86_64",
            None,
            current,
        )?)
    }

    #[test]
    fn a_newer_release_is_available_with_its_link() -> TestResult {
        let status = judged("0.2.0", "https://example.com/setup.exe", "0.1.0")?;
        assert!(matches!(&status, Status::Available(update) if update.version == "0.2.0"));
        Ok(())
    }

    #[test]
    fn the_same_or_an_older_release_is_current() -> TestResult {
        assert_eq!(
            judged("0.1.0", "https://example.com/setup.exe", "0.1.0")?,
            Status::Current
        );
        assert_eq!(
            judged("0.1.0", "https://example.com/setup.exe", "0.2.0")?,
            Status::Current
        );
        Ok(())
    }

    #[test]
    fn a_plain_http_link_is_refused() -> TestResult {
        let status = judged("0.2.0", "http://example.com/setup.exe", "0.1.0")?;
        assert!(matches!(status, Status::Failed(_)));
        Ok(())
    }

    #[test]
    fn a_malformed_version_is_an_error() {
        assert!(judged("soon", "https://example.com/setup.exe", "0.1.0").is_err());
    }

    #[test]
    fn a_checker_reports_the_status_it_was_given() {
        assert_eq!(
            Checker::with_status(Status::Current).status(),
            Status::Current
        );
        let update = Update {
            version: "0.2.0".to_owned(),
            url: "https://example.com/setup.exe".to_owned(),
            sha256: String::new(),
        };
        assert_eq!(
            Checker::with_status(Status::Available(update.clone())).status(),
            Status::Available(update)
        );
    }

    #[test]
    fn civil_dates_match_the_calendar() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(civil(19_723), (2024, 1, 1));
        assert_eq!(civil(19_782), (2024, 2, 29));
    }

    #[test]
    fn later_dates_sort_after_today() {
        let today = date(0);
        assert_eq!(today.len(), 10);
        assert!(date(super::REMIND_DAYS) > today);
    }
}
