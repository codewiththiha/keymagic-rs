//! Throughput of typing a Myanmar paragraph on the myWin keyboard.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use keymagic_engine::{Engine, KeyEvent};
use std::path::Path;
use std::sync::Arc;

const PARAGRAPH: &str = include_str!("myanmar_paragraph.txt");

fn paragraph_keys() -> Vec<KeyEvent> {
    PARAGRAPH
        .chars()
        .map(|ch| KeyEvent {
            ch: Some(ch),
            ..KeyEvent::default()
        })
        .collect()
}

fn typing_throughput(criterion: &mut Criterion) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/km2/alternatives/myWin.km2");
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => panic!("reading the benchmark keyboard: {error}"),
    };
    let keyboard = match keymagic_km2::parse(&bytes) {
        Ok(keyboard) => keyboard,
        Err(error) => panic!("parsing the benchmark keyboard: {error}"),
    };
    let mut engine = match Engine::new(Arc::new(keyboard)) {
        Ok(engine) => engine,
        Err(error) => panic!("building the benchmark engine: {error}"),
    };
    let keys = paragraph_keys();
    let mut group = criterion.benchmark_group("engine");
    group.throughput(Throughput::Elements(u64::try_from(keys.len()).unwrap_or(0)));
    group.bench_function("typing_myanmar_paragraph", |bencher| {
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
