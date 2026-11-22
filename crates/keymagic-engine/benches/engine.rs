//! Throughput of typing a seeded 2,000-key stream on the myWin keyboard.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use keymagic_engine::{Engine, KeyEvent};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;

const KEY_COUNT: usize = 2_000;

fn key_stream() -> Vec<KeyEvent> {
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    (0..KEY_COUNT)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let offset = u8::try_from(state % 26).unwrap_or(0);
            KeyEvent {
                ch: Some(char::from(b'a' + offset)),
                ..KeyEvent::default()
            }
        })
        .collect()
}

fn load_keyboard() -> Result<keymagic_types::Keyboard, Box<dyn Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/km2/alternatives/myWin.km2");
    Ok(keymagic_km2::parse(&std::fs::read(path)?)?)
}

fn typing_throughput(criterion: &mut Criterion) {
    let keyboard = match load_keyboard() {
        Ok(keyboard) => keyboard,
        Err(error) => panic!("loading the benchmark keyboard: {error}"),
    };
    let mut engine = match Engine::new(Arc::new(keyboard)) {
        Ok(engine) => engine,
        Err(error) => panic!("building the benchmark engine: {error}"),
    };
    let keys = key_stream();
    let mut group = criterion.benchmark_group("engine");
    group.throughput(Throughput::Elements(2_000));
    group.bench_function("typing_2000_keys", |bencher| {
        bencher.iter(|| {
            engine.reset();
            for key in &keys {
                black_box(engine.process(*key));
            }
        });
    });
    group.finish();
}

criterion_group!(benches, typing_throughput);
criterion_main!(benches);
