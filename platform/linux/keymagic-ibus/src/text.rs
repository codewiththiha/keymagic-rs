//! Builds the `IBusText` values that `IBus` signals carry.

use std::collections::HashMap;
use zbus::{OwnedVariant, Structure, Variant};

/// Builds an `IBusText` (`(sa{sv}sv)`) holding `text` with no attributes.
#[must_use]
pub fn ibus_text(text: &str) -> Variant<'static> {
    let fields = (
        "IBusText",
        HashMap::<String, OwnedVariant>::new(),
        text.to_owned(),
        attribute_list(),
    );
    Variant::Structure(Structure::from(fields))
}

fn attribute_list() -> Variant<'static> {
    let fields = (
        "IBusAttrList",
        HashMap::<String, OwnedVariant>::new(),
        Vec::<OwnedVariant>::new(),
    );
    Variant::Structure(Structure::from(fields))
}

#[cfg(test)]
mod tests {
    use super::ibus_text;

    #[test]
    fn ibus_text_has_the_ibus_layout() {
        let value = ibus_text("ab");
        assert_eq!(value.value_signature().to_string(), "(sa{sv}sv)");
    }
}
