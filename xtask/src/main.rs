//! Synchronize pinned corpus fixtures from their upstream Git repository.

#![forbid(unsafe_code)]

use std::{
    collections::HashSet,
    env, fs, io,
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(test)]
mod tests;

struct Fixture {
    upstream: String,
    destination: PathBuf,
    blob: String,
}

struct Manifest {
    repository: String,
    revision: String,
    files: Vec<Fixture>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Write,
    DryRun,
    Check,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn safe_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['\\', ':'])
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

impl Manifest {
    fn parse(input: &str) -> io::Result<Self> {
        let mut lines = input.lines();
        let header: Vec<_> = lines
            .next()
            .ok_or_else(|| invalid("missing source"))?
            .split('\t')
            .collect();
        if header.len() != 3
            || header[0] != "source"
            || !header[1].starts_with("https://")
            || !hex(header[2], 40)
        {
            return Err(invalid("invalid source header"));
        }
        let mut files = Vec::new();
        let mut destinations = HashSet::new();
        for line in lines {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 5
                || fields[0] != "file"
                || !safe_path(fields[1])
                || !safe_path(fields[2])
                || !hex(fields[3], 40)
                || !hex(fields[4], 64)
            {
                return Err(invalid("invalid fixture entry"));
            }
            if !destinations.insert(fields[2].to_owned()) {
                return Err(invalid("duplicate fixture destination"));
            }
            files.push(Fixture {
                upstream: fields[1].to_owned(),
                destination: PathBuf::from(fields[2]),
                blob: fields[3].to_owned(),
            });
        }
        if files.is_empty() {
            return Err(invalid("no fixtures in manifest"));
        }
        Ok(Self {
            repository: header[1].to_owned(),
            revision: header[2].to_owned(),
            files,
        })
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> io::Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        for attempt in 0..100 {
            let path = env::temp_dir().join(format!(
                "keymagic-corpus-{}-{nanos}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "cannot create temporary directory",
        ))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(repo: &Path, args: &[&str]) -> io::Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout)
}

fn sync(manifest: &Manifest, destination: &Path, mode: Mode) -> io::Result<usize> {
    let scratch = Scratch::new()?;
    git(scratch.path(), &["init", "--bare", "--quiet"])?;
    git(
        scratch.path(),
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            "--depth=1",
            "--",
            &manifest.repository,
            &manifest.revision,
        ],
    )?;
    let mut changes = Vec::new();
    for fixture in &manifest.files {
        let object = format!("{}:{}", manifest.revision, fixture.upstream);
        let blob = git(scratch.path(), &["rev-parse", "--verify", &object])?;
        if String::from_utf8_lossy(&blob).trim() != fixture.blob {
            return Err(invalid("upstream Git blob does not match manifest"));
        }
        let bytes = git(scratch.path(), &["show", &object])?;
        let path = destination.join(&fixture.destination);
        let existing = match fs::read(&path) {
            Ok(data) => Some(data),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        if existing.as_deref() != Some(bytes.as_slice()) {
            changes.push((path, bytes));
        }
    }
    for (path, _) in &changes {
        println!("{}", path.display());
    }
    if mode == Mode::Check && !changes.is_empty() {
        return Err(invalid("corpus differs from pinned upstream"));
    }
    if mode == Mode::Write {
        for (path, bytes) in &changes {
            let parent = path
                .parent()
                .ok_or_else(|| invalid("fixture has no parent"))?;
            fs::create_dir_all(parent)?;
            fs::write(path, bytes)?;
        }
    }
    Ok(changes.len())
}

fn gen_headers(root: &Path) -> io::Result<()> {
    let crate_dir = root.join("crates").join("keymagic-ffi");
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .map_err(|error| invalid(&error))?;
    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .map_err(|error| invalid(&error.to_string()))?;
    let header = crate_dir.join("include").join("keymagic.h");
    if let Some(parent) = header.parent() {
        fs::create_dir_all(parent)?;
    }
    bindings.write_to_file(header);
    Ok(())
}

fn run() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| invalid("workspace root missing"))?;
    let mode = match args.as_slice() {
        [command] if command == "gen-headers" => return gen_headers(root),
        [command] if command == "corpus-sync" => Mode::Write,
        [command, flag] if command == "corpus-sync" && flag == "--dry-run" => Mode::DryRun,
        [command, flag] if command == "corpus-sync" && flag == "--check" => Mode::Check,
        _ => {
            return Err(invalid(
                "usage: cargo xtask [gen-headers | corpus-sync [--dry-run|--check]]",
            ));
        }
    };
    let corpus = root.join("corpus");
    let mut count = 0;
    for name in [
        "sources.tsv",
        "alternatives-mit.tsv",
        "alternatives-lgpl.tsv",
        "alternatives-unlicensed.tsv",
    ] {
        let manifest = Manifest::parse(&fs::read_to_string(corpus.join(name))?)?;
        count += sync(&manifest, &corpus, mode)?;
    }
    println!("{count} fixture(s) differ from the pinned source");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}
