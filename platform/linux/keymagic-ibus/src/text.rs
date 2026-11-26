//! Builds the `IBus` values that signals carry: texts, properties and lists.

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

/// Builds an `IBusProperty` (`(sa{sv}suvsvbbuvv)`) with no icon or tooltip.
#[must_use]
pub fn property(
    key: &str,
    kind: u32,
    label: &str,
    state: u32,
    sub_props: Variant<'static>,
) -> Variant<'static> {
    let fields = (
        "IBusProperty",
        HashMap::<String, OwnedVariant>::new(),
        key.to_owned(),
        kind,
        ibus_text(label),
        String::new(),
        ibus_text(""),
        true,
        true,
        state,
        sub_props,
        ibus_text(""),
    );
    Variant::Structure(Structure::from(fields))
}

/// Builds an `IBusPropList` (`(sa{sv}av)`) holding `props`.
pub fn prop_list(props: &[Variant<'static>]) -> zbus::Result<Variant<'static>> {
    let items = props
        .iter()
        .map(Variant::try_to_owned)
        .collect::<zbus::Result<Vec<OwnedVariant>>>()?;
    let fields = ("IBusPropList", HashMap::<String, OwnedVariant>::new(), items);
    Ok(Variant::Structure(Structure::from(fields)))
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
    use super::{ibus_text, prop_list, property};

    #[test]
    fn ibus_text_has_the_ibus_layout() {
        let value = ibus_text("ab");
        assert_eq!(value.value_signature().to_string(), "(sa{sv}sv)");
    }

    #[test]
    fn property_has_the_ibus_layout() {
        let Ok(sub_props) = prop_list(&[]) else {
            panic!("an empty list must build");
        };
        let value = property("key", 2, "label", 1, sub_props);
        assert_eq!(value.value_signature().to_string(), "(sa{sv}suvsvbbuvv)");
    }

    #[test]
    fn prop_list_has_the_ibus_layout() {
        let Ok(list) = prop_list(&[ibus_text("x")]) else {
            panic!("a list of texts must build");
        };
        assert_eq!(list.value_signature().to_string(), "(sa{sv}av)");
    }
}
