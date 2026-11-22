//! Throughput of typing a Myanmar paragraph on the myWin keyboard.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use keymagic_engine::{Engine, KeyEvent};
use std::sync::Arc;

const PARAGRAPH: &str = include_str!("myanmar_paragraph.txt");
const KEYBOARD: &[u8] = include_bytes!("../../../corpus/km2/alternatives/myWin.km2");

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
    let Ok(keyboard) = keymagic_km2::parse(KEYBOARD) else {
        panic!("parsing the benchmark keyboard");
    };
    let Ok(mut engine) = Engine::new(Arc::new(keyboard)) else {
        panic!("building the benchmark engine");
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
