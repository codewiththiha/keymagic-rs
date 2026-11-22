//! Seeded key streams replayed through the pinned upstream driver.

mod common;

use keymagic_engine::{Engine, KeyEvent, Modifiers, Output, VirtualKey};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// Seeded streams per keyboard.
const STREAMS: u64 = 5_000;

/// Streams per driver process, keeping argument lists small.
const STREAMS_PER_RUN: u64 = 100;

/// Upper bound on random steps after the initial reset of a stream.
const MAX_STEPS: u64 = 24;

const NO_MODIFIERS: [bool; 4] = [false; 4];

/// Virtual-key codes used by corpus rules, plus Back.
const KEY_CODES: &[u16] = &[
    2, 5, 6, 7, 32, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 80, 81, 82, 83, 84, 85, 112, 113, 114,
    115, 116, 117, 118, 119, 120, 121, 122, 123, 186, 187, 188, 189, 190, 191,
];

/// Printable ASCII plus Myanmar letters, signs and combining marks.
const CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 !\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~\u{1000}\u{1001}\u{1004}\u{103B}\u{103C}\u{102B}\u{103A}\u{1031}\u{301}";

#[derive(Deserialize)]
struct CorpusFile {
    case: Vec<KeyboardRef>,
}

#[derive(Deserialize)]
struct KeyboardRef {
    keyboard: String,
}

/// One scripted step applied to both engines.
enum Step {
    Reset,
    SetComposing(String),
    Key {
        key: Option<u16>,
        mods: [bool; 4],
        ch: Option<char>,
        preview: bool,
    },
}

/// Deterministic xorshift generator.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    const fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    const fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        let bound = u64::try_from(items.len()).unwrap_or(u64::MAX);
        items[usize::try_from(self.below(bound)).unwrap_or(0)]
    }

    const fn flag(&mut self) -> bool {
        self.below(2) == 0
    }

    const fn preview(&mut self) -> bool {
        self.below(5) == 0
    }
}

const fn splitmix(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn random_step(rng: &mut Rng, pool: &[char]) -> Step {
    match rng.below(100) {
        0..=1 => Step::Reset,
        2..=3 => Step::SetComposing((0..rng.below(4)).map(|_| rng.pick(pool)).collect()),
        4..=49 => Step::Key {
            key: None,
            mods: NO_MODIFIERS,
            ch: Some(rng.pick(pool)),
            preview: rng.preview(),
        },
        50..=79 => Step::Key {
            key: Some(rng.pick(KEY_CODES)),
            mods: NO_MODIFIERS,
            ch: Some(rng.pick(pool)),
            preview: rng.preview(),
        },
        80..=94 => Step::Key {
            key: Some(if rng.below(3) == 0 {
                2
            } else {
                rng.pick(KEY_CODES)
            }),
            mods: NO_MODIFIERS,
            ch: None,
            preview: rng.preview(),
        },
        _ => Step::Key {
            key: Some(rng.pick(KEY_CODES)),
            mods: [rng.flag(), rng.flag(), rng.flag(), rng.flag()],
            ch: if rng.flag() {
                Some(rng.pick(pool))
            } else {
                None
            },
            preview: rng.preview(),
        },
    }
}

/// Builds one stream: a reset, then seeded random steps.
fn stream(index: u64, pool: &[char]) -> Vec<Step> {
    let mut rng = Rng::new(splitmix(index));
    let mut steps = vec![Step::Reset];
    steps.extend((0..=rng.below(MAX_STEPS)).map(|_| random_step(&mut rng, pool)));
    steps
}

fn modifier_mask(mods: [bool; 4]) -> u8 {
    [1_u8, 2, 4, 8]
        .iter()
        .zip(mods)
        .filter(|(_, set)| *set)
        .map(|(bit, _)| *bit)
        .sum()
}

/// Encodes one step in the reference driver's argument protocol.
fn encode(step: &Step) -> Result<String, Box<dyn Error>> {
    Ok(match step {
        Step::Reset => "r:".to_string(),
        Step::SetComposing(text) => format!("s:{text}"),
        Step::Key {
            key,
            mods,
            ch,
            preview,
        } => {
            let mask = modifier_mask(*mods);
            let body = match (key, ch, mask) {
                (Some(vk), _, flags) if flags != 0 => {
                    format!(
                        "m:{flags}:{vk}:{}",
                        ch.map_or_else(String::new, String::from)
                    )
                }
                (Some(vk), Some(c), 0) => format!("k:{vk}:{c}"),
                (Some(vk), None, 0) => format!("v:{vk}"),
                (None, Some(c), 0) => format!("c:{c}"),
                _ => return Err("step has no key and no character".into()),
            };
            if *preview { format!("t:{body}") } else { body }
        }
    })
}

fn unhex(text: &str) -> Result<String, Box<dyn Error>> {
    if !text.len().is_multiple_of(2) {
        return Err("odd hex length".into());
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    for index in (0..text.len()).step_by(2) {
        let pair = text.get(index..index + 2).ok_or("invalid hex")?;
        bytes.push(u8::from_str_radix(pair, 16)?);
    }
    Ok(String::from_utf8(bytes)?)
}

/// One `CORPUS` line printed by the driver.
struct Reported {
    action: String,
    count: usize,
    text: String,
    composing: String,
    processed: bool,
    engine_composing: String,
}

fn parse_line(line: &str) -> Result<Reported, Box<dyn Error>> {
    let fields: Vec<&str> = line.split('\t').collect();
    let [tag, action, count, text, composing, processed, engine] = fields.as_slice() else {
        return Err(format!("malformed reference line: {line}").into());
    };
    if *tag != "CORPUS" {
        return Err(format!("unexpected reference line: {line}").into());
    }
    Ok(Reported {
        action: (*action).to_string(),
        count: count.parse()?,
        text: unhex(text)?,
        composing: unhex(composing)?,
        processed: match *processed {
            "true" => true,
            "false" => false,
            _ => return Err("invalid processed flag".into()),
        },
        engine_composing: unhex(engine)?,
    })
}

fn key_event(key: Option<u16>, mods: [bool; 4], ch: Option<char>) -> KeyEvent {
    KeyEvent {
        key: key.and_then(VirtualKey::from_u16),
        ch,
        mods: Modifiers {
            shift: mods[0],
            ctrl: mods[1],
            alt: mods[2],
            caps_lock: mods[3],
        },
    }
}

fn compare_output(reported: &Reported, output: &Output, engine: &Engine) -> Result<(), String> {
    let (action, count, text) = common::action_fields(&output.action);
    let observed = (action, count, text.as_str(), output.consumed);
    let expected = (
        reported.action.as_str(),
        reported.count,
        reported.text.as_str(),
        reported.processed,
    );
    if observed != expected {
        return Err(format!("action: expected {expected:?}, got {observed:?}"));
    }
    if output.composing != reported.composing {
        return Err(format!(
            "composing: expected {:?}, got {:?}",
            reported.composing, output.composing
        ));
    }
    if engine.composing() != reported.engine_composing {
        return Err(format!(
            "engine composing: expected {:?}, got {:?}",
            reported.engine_composing,
            engine.composing()
        ));
    }
    Ok(())
}

/// Replays one stream (it begins with a reset) and checks each reported line.
fn replay(engine: &mut Engine, steps: &[Step], reported: &[Reported]) -> Result<(), String> {
    for (index, (step, line)) in steps.iter().zip(reported).enumerate() {
        let context = |error: String| format!("step {index}: {error}");
        match step {
            Step::Reset => {
                engine.reset();
                if line.composing != engine.composing() {
                    return Err(context("reset composing differs".into()));
                }
            }
            Step::SetComposing(text) => {
                engine.set_composing(text);
                if line.composing != engine.composing() {
                    return Err(context("set composing differs".into()));
                }
            }
            Step::Key {
                key,
                mods,
                ch,
                preview,
            } => {
                let event = key_event(*key, *mods, *ch);
                let output = if *preview {
                    engine.peek(event)
                } else {
                    engine.process(event)
                };
                compare_output(line, &output, engine).map_err(context)?;
            }
        }
    }
    Ok(())
}

fn corpus_keyboards() -> Result<Vec<String>, Box<dyn Error>> {
    let mut keyboards = BTreeSet::new();
    for entry in fs::read_dir(common::corpus_dir().join("cases"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            let file: CorpusFile = toml::from_str(&fs::read_to_string(&path)?)?;
            keyboards.extend(file.case.into_iter().map(|case| case.keyboard));
        }
    }
    Ok(keyboards.into_iter().collect())
}

fn run_driver(
    driver: &Path,
    keyboard: &Path,
    arguments: &[String],
) -> Result<String, Box<dyn Error>> {
    let output = Command::new(driver)
        .arg(keyboard)
        .args(arguments)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "reference driver failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// Checks every seeded stream for one keyboard; returns steps compared.
fn check_keyboard(
    driver: &Path,
    relative: &str,
    keyboard_index: u64,
    pool: &[char],
) -> Result<u64, Box<dyn Error>> {
    let path = common::keyboard_path(relative)?;
    let keyboard = Arc::new(keymagic_km2::parse(&fs::read(&path)?)?);
    let mut engine = Engine::new(keyboard)?;
    let mut compared = 0_u64;
    for batch in 0..STREAMS / STREAMS_PER_RUN {
        let mut arguments = Vec::new();
        let mut streams = Vec::new();
        for offset in 0..STREAMS_PER_RUN {
            let index = keyboard_index * STREAMS + batch * STREAMS_PER_RUN + offset;
            let steps = stream(index, pool);
            for step in &steps {
                arguments.push(encode(step)?);
            }
            streams.push(steps);
        }
        let lines: Vec<Reported> = run_driver(driver, &path, &arguments)?
            .lines()
            .filter(|line| line.starts_with("CORPUS\t"))
            .map(parse_line)
            .collect::<Result<_, _>>()?;
        if lines.len() != arguments.len() {
            return Err(format!(
                "{relative}: expected {} reference lines, got {}",
                arguments.len(),
                lines.len()
            )
            .into());
        }
        let mut consumed = 0;
        for (offset, steps) in streams.iter().enumerate() {
            let reported = &lines[consumed..consumed + steps.len()];
            let stream_number = batch * STREAMS_PER_RUN + u64::try_from(offset)?;
            replay(&mut engine, steps, reported)
                .map_err(|error| format!("{relative} stream {stream_number}: {error}"))?;
            consumed += steps.len();
            compared += u64::try_from(steps.len())?;
        }
    }
    Ok(compared)
}

#[test]
#[ignore = "needs the pinned reference driver and KM2 directory"]
fn seeded_streams_match_upstream_on_every_corpus_keyboard() -> Result<(), Box<dyn Error>> {
    let driver = PathBuf::from(env::var("KEYMAGIC_REFERENCE_DRIVER")?);
    let pool: Vec<char> = CHARS.chars().collect();
    let mut compared = 0_u64;
    for (index, relative) in corpus_keyboards()?.iter().enumerate() {
        compared += check_keyboard(&driver, relative, u64::try_from(index)?, &pool)?;
    }
    assert!(compared > 0);
    Ok(())
}
