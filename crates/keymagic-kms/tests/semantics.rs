//! Lexer, parser and lowering rules checked on small inline scripts.

use keymagic_kms::{Diagnostic, Keyboard, Resolver, Severity, check, compile, lexer};
use keymagic_types::{OutputElement, PatternElement};
use std::collections::HashMap;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Default)]
struct Files(HashMap<String, Vec<u8>>);

impl Resolver for Files {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0.get(path).cloned()
    }
}

fn build(src: &str) -> Result<Keyboard, Vec<Diagnostic>> {
    compile(src, &Files::default())
}

fn messages(src: &str) -> Vec<String> {
    match build(src) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.into_iter().map(|error| error.message).collect(),
    }
}

#[test]
fn unicode_literals_need_exactly_four_hex_digits() -> TestResult {
    let kinds: Vec<lexer::Kind> = lexer::lex("u1000 U1000x U")?
        .tokens
        .into_iter()
        .map(|token| token.kind)
        .collect();
    assert_eq!(kinds[0], lexer::Kind::Uni(0x1000));
    assert_eq!(kinds[1], lexer::Kind::Ident("U1000x".to_owned()));
    assert_eq!(kinds[2], lexer::Kind::Ident("U".to_owned()));
    Ok(())
}

#[test]
fn line_continuation_joins_value_lines() -> TestResult {
    let keyboard =
        build("$v = \"a\" + \\\n    \"b\"\n\"x\" => $v\n").map_err(|e| format!("{e:?}"))?;
    assert_eq!(keyboard.strings, vec!["ab".to_owned()]);
    Ok(())
}

#[test]
fn unterminated_string_is_a_lexer_error() {
    assert_eq!(messages("\"abc => \"x\"\n"), vec!["unterminated string"]);
}

#[test]
fn states_are_numbered_by_first_use() -> TestResult {
    let keyboard = build("('one') + \"a\" => ('two')\n('two') + \"b\" => \"c\"\n")
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(keyboard.rules[0].lhs[0], PatternElement::State(0));
    assert_eq!(keyboard.rules[0].rhs[0], OutputElement::State(1));
    assert_eq!(keyboard.rules[1].lhs[0], PatternElement::State(1));
    Ok(())
}

#[test]
fn back_references_must_name_a_capture() {
    assert_eq!(
        messages("\"a\" => $2\n"),
        vec!["$2 is not a capture of the left side"]
    );
    assert_eq!(
        messages("\"a\" => $0\n"),
        vec!["$0 is not a capture of the left side"]
    );
}

#[test]
fn undefined_variables_are_errors() {
    assert_eq!(messages("$x => \"a\"\n"), vec!["undefined variable '$x'"]);
}

#[test]
fn null_and_any_are_limited_to_their_side() {
    assert!(messages("NULL => \"a\"\n")[0].contains("NULL is only allowed"));
    assert!(messages("\"a\" => ANY\n")[0].contains("ANY is only allowed"));
}

#[test]
fn unknown_virtual_key_suggests_the_nearest_name() -> TestResult {
    let errors = build("<VK_SHFT> => \"a\"\n")
        .err()
        .ok_or("expected an error")?;
    assert_eq!(errors[0].help.as_deref(), Some("did you mean 'VK_SHIFT'?"));
    Ok(())
}

#[test]
fn the_last_option_value_wins() -> TestResult {
    let keyboard = build(
        "/*\n@TRACK_CAPSLOCK = \"TRUE\"\n*/\n// @TRACK_CAPSLOCK = \"FALSE\"\n\"a\" => \"b\"\n",
    )
    .map_err(|e| format!("{e:?}"))?;
    assert!(!keyboard.options.track_caps);
    Ok(())
}

#[test]
fn name_and_hotkey_become_info_records() -> TestResult {
    let keyboard = build("// @NAME = \"Demo\"\n// @HOTKEY = \"CTRL+ALT+W\"\n\"a\" => \"b\"\n")
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(keyboard.info.name(), Some("Demo"));
    assert_eq!(keyboard.info.hotkey(), Some("CTRL+ALT+W"));
    Ok(())
}

#[test]
fn icons_must_be_images_read_through_the_resolver() -> TestResult {
    let png = b"\x89PNG\r\n\x1a\nxxxx".to_vec();
    let mut files = Files::default();
    files.0.insert("icon.png".to_owned(), png.clone());
    let keyboard = compile("// @ICON = \"icon.png\"\n\"a\" => \"b\"\n", &files)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(keyboard.info.icon(), Some(png.as_slice()));

    let mut text = Files::default();
    text.0
        .insert("icon.png".to_owned(), b"not an image at all".to_vec());
    let errors = compile("// @ICON = \"icon.png\"\n\"a\" => \"b\"\n", &text)
        .err()
        .ok_or("expected an error")?;
    assert!(errors[0].message.contains("PNG, BMP or JPEG"));
    Ok(())
}

#[test]
fn circular_includes_are_reported() -> TestResult {
    let mut files = Files::default();
    files.0.insert(
        "a.kms".to_owned(),
        b"include(\"b.kms\")\n\"a\" => \"b\"\n".to_vec(),
    );
    files.0.insert(
        "b.kms".to_owned(),
        b"include(\"a.kms\")\n\"c\" => \"d\"\n".to_vec(),
    );
    let errors = compile("include(\"a.kms\")\n", &files)
        .err()
        .ok_or("expected an error")?;
    assert!(errors[0].message.starts_with("circular include"));
    Ok(())
}

#[test]
fn repeated_variables_and_unread_states_are_warnings() {
    let found = check(
        "$v = \"a\"\n$v = \"b\"\n\"x\" => ('lonely')\n",
        &Files::default(),
    );
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|d| d.severity == Severity::Warning));
}

#[test]
fn option_pairs_are_read_from_the_first_block_and_slash_lines() {
    let pairs: Vec<(String, String)> =
        lexer::options("/*\n@NAME = \"X\"\n*/\n// @EAT_KEYS = \"TRUE\"\n")
            .into_iter()
            .map(|setting| (setting.key, setting.value))
            .collect();
    assert_eq!(
        pairs,
        vec![
            ("NAME".to_owned(), "X".to_owned()),
            ("EAT_KEYS".to_owned(), "TRUE".to_owned()),
        ]
    );
}

#[test]
fn option_spans_point_at_their_line() {
    let src = "/*\n@NAME = \"X\"\n*/\n// @EAT_KEYS = \"TRUE\"\n";
    let texts: Vec<&str> = lexer::options(src)
        .iter()
        .map(|setting| &src[setting.span.clone()])
        .collect();
    assert_eq!(texts, vec!["@NAME = \"X\"", "// @EAT_KEYS = \"TRUE\""]);
}
