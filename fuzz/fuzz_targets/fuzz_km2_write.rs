//! Generate varied keyboard structures for checked serialization.

#![no_main]
#![forbid(unsafe_code)]

use keymagic_km2::{parse, write};
use keymagic_types::{
    InfoEntry, Keyboard, KeyboardInfo, LayoutOptions, OutputElement, PatternElement, Rule,
    VirtualKey,
};
use libfuzzer_sys::fuzz_target;

struct Input<'a> {
    bytes: &'a [u8],
    index: usize,
}
impl Input<'_> {
    fn next(&mut self) -> u8 {
        let byte = self.bytes.get(self.index).copied().unwrap_or(0);
        self.index = self.index.saturating_add(1);
        byte
    }
    fn text(&mut self) -> String {
        let len = usize::from(self.next() % 16);
        let mut data = Vec::new();
        for _ in 0..len {
            data.push(self.next());
        }
        String::from_utf8_lossy(&data).into_owned()
    }
}

fuzz_target!(|data: &[u8]| {
    let mut input = Input {
        bytes: data,
        index: 0,
    };
    let options = LayoutOptions {
        track_caps: input.next() % 2 != 0,
        auto_bksp: input.next() % 2 != 0,
        eat: input.next() % 2 != 0,
        pos_based: input.next() % 2 != 0,
        right_alt: input.next() % 2 != 0,
    };
    let string_count = input.next() % 6;
    let mut strings = Vec::new();
    for _ in 0..string_count {
        strings.push(input.text());
    }
    let info_count = input.next() % 4;
    let mut entries = Vec::new();
    for _ in 0..info_count {
        let id = [input.next(), input.next(), input.next(), input.next()];
        entries.push(InfoEntry {
            id,
            data: input.text().into_bytes(),
        });
    }
    let rule_count = input.next() % 8;
    let mut rules = Vec::new();
    for _ in 0..rule_count {
        let lhs_count = input.next() % 8;
        let mut lhs = Vec::new();
        for _ in 0..lhs_count {
            let variant = input.next() % 7;
            let v = usize::from(input.next() % 8);
            lhs.push(match variant {
                0 => PatternElement::Text(input.text()),
                1 => PatternElement::Variable(v),
                2 => PatternElement::AnyOf(v),
                3 => PatternElement::NotAnyOf(v),
                4 => PatternElement::Any,
                5 => PatternElement::VirtualKeys(vec![
                    VirtualKey::from_u16(u16::try_from(v + 1).unwrap_or(1))
                        .unwrap_or(VirtualKey::Null),
                ]),
                _ => PatternElement::State(u16::try_from(v).unwrap_or(0)),
            });
        }
        let rhs_count = input.next() % 8;
        let mut rhs = Vec::new();
        for _ in 0..rhs_count {
            let variant = input.next() % 6;
            let v = usize::from(input.next() % 8);
            rhs.push(match variant {
                0 => OutputElement::Text(input.text()),
                1 => OutputElement::Variable(v),
                2 => OutputElement::Ref(v),
                3 => OutputElement::VarIndexed {
                    var: v,
                    by_ref: usize::from(input.next() % 8),
                },
                4 => OutputElement::Null,
                _ => OutputElement::State(u16::try_from(v).unwrap_or(0)),
            });
        }
        rules.push(Rule {
            lhs,
            rhs,
            source: None,
        });
    }
    let keyboard = Keyboard {
        options,
        strings,
        info: KeyboardInfo { entries },
        rules,
    };
    if let Ok(bytes) = write(&keyboard) {
        assert_eq!(parse(&bytes).ok(), Some(keyboard));
    }
});
