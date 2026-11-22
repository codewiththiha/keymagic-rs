#![no_main]

use keymagic_engine::{Action, Engine, KeyEvent, Modifiers, VirtualKey};
use libfuzzer_sys::fuzz_target;
use std::sync::Arc;

const KEYBOARDS: [&[u8]; 5] = [
    include_bytes!("../../corpus/km2/upstream/comprehensive_test.km2"),
    include_bytes!("../../corpus/km2/alternatives/Eastern-Pwo_kjp.km2"),
    include_bytes!("../../corpus/km2/alternatives/Sgaw-Kawthoolei_ksw.km2"),
    include_bytes!("../../corpus/km2/alternatives/Western-Pwo_pwo.km2"),
    include_bytes!("../../corpus/km2/alternatives/myWin.km2"),
];

fn decode(kind: u8, value: u8) -> KeyEvent {
    let mods = Modifiers {
        shift: kind & 0b0100 != 0,
        ctrl: kind & 0b1000 != 0,
        alt: kind & 0b0001_0000 != 0,
        caps_lock: kind & 0b0010_0000 != 0,
    };
    let ch = match kind & 0b11 {
        0 => Some(char::from(0x20 + value % 95)),
        2 => char::from_u32(0x1000 + u32::from(value)),
        _ => None,
    };
    let key = if kind & 0b11 == 0 {
        None
    } else {
        VirtualKey::from_u16(u16::from(value))
    };
    KeyEvent { key, ch, mods }
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    let index = usize::from(selector) % KEYBOARDS.len();
    let Ok(keyboard) = keymagic_km2::parse(KEYBOARDS[index]) else {
        return;
    };
    let Ok(mut engine) = Engine::new(Arc::new(keyboard)) else {
        return;
    };
    for chunk in rest.chunks_exact(2) {
        let (kind, value) = (chunk[0], chunk[1]);
        if kind & 0b11 == 3 {
            if value == 0 {
                engine.reset();
            } else {
                engine.set_composing(&char::from(0x20 + value % 95).to_string());
            }
            continue;
        }
        let event = decode(kind, value);
        let before = engine.composing().chars().count();
        let peeked = engine.peek(event);
        let output = engine.process(event);
        assert_eq!(peeked, output);
        assert_eq!(output.composing, engine.composing());
        if let Action::Delete(count) | Action::Replace { delete: count, .. } = output.action {
            assert!(count <= before);
        }
    }
});
