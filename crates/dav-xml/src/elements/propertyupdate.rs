// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::element::ElementExt;
use crate::elements::{Prop, Remove, Set};
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// A `set` or `remove` item in a [`PropertyUpdate`].
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyUpdateItem {
    /// Set the contained properties.
    Set(Set),
    /// Remove the contained properties.
    Remove(Remove),
}

/// The `propertyupdate` XML element as defined in [RFC 4918 section 14.20](https://www.rfc-editor.org/rfc/rfc4918#section-14.20).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::{Prop, PropertyUpdate};
/// use dav_xml::properties::DisplayName;
///
/// let update = PropertyUpdate::new()
///     .set(Prop::builder().property(DisplayName("Notes".into())).build())
///     .remove(Prop::builder().name::<DisplayName>().build());
/// assert_eq!(update.0.len(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PropertyUpdate(pub Vec<PropertyUpdateItem>);

impl Element for PropertyUpdate {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "propertyupdate";
}

impl PropertyUpdate {
    /// Create an empty property update.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::PropertyUpdate;
    ///
    /// let update = PropertyUpdate::new();
    /// assert!(update.0.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a `set` instruction.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::{Prop, PropertyUpdate};
    /// use dav_xml::properties::DisplayName;
    ///
    /// let update = PropertyUpdate::new()
    ///     .set(Prop::builder().property(DisplayName("Notes".into())).build());
    /// assert_eq!(update.0.len(), 1);
    /// ```
    #[must_use]
    pub fn set(mut self, prop: Prop) -> Self {
        self.0.push(PropertyUpdateItem::Set(Set(prop)));
        self
    }

    /// Append a `remove` instruction.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::{Prop, PropertyUpdate};
    /// use dav_xml::properties::DisplayName;
    ///
    /// let update = PropertyUpdate::new()
    ///     .remove(Prop::builder().name::<DisplayName>().build());
    /// assert_eq!(update.0.len(), 1);
    /// ```
    #[must_use]
    pub fn remove(mut self, prop: Prop) -> Self {
        self.0.push(PropertyUpdateItem::Remove(Remove(prop)));
        self
    }
}

impl TryFrom<&Value> for PropertyUpdate {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;
        let mut items = Vec::new();

        for (name, child) in map.iter_ordered() {
            if name == &Set::element_name::<&str>() {
                items.push(PropertyUpdateItem::Set(Set::try_from(child)?));
            } else if name == &Remove::element_name::<&str>() {
                items.push(PropertyUpdateItem::Remove(Remove::try_from(child)?));
            }
        }

        if items.is_empty() {
            return Err(Error::MissingElement {
                parent: Self::LOCAL_NAME,
                element: "set or remove",
            });
        }

        Ok(Self(items))
    }
}

impl From<PropertyUpdate> for Value {
    fn from(PropertyUpdate(items): PropertyUpdate) -> Self {
        let mut map = ValueMap::new();
        for item in items {
            match item {
                PropertyUpdateItem::Set(set) => {
                    map.insert_raw(Set::element_name(), set.into());
                }
                PropertyUpdateItem::Remove(remove) => {
                    map.insert_raw(Remove::element_name(), remove.into());
                }
            }
        }
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::properties::DisplayName;
    use crate::{FromXml, IntoXml};

    const RFC_9_2_2: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:propertyupdate xmlns:D="DAV:" xmlns:Z="http://ns.example.com/z/">
  <D:set>
    <D:prop>
      <Z:Authors>
        <Z:Author>Jim Whitehead</Z:Author>
        <Z:Author>Roy Fielding</Z:Author>
      </Z:Authors>
    </D:prop>
  </D:set>
  <D:remove>
    <D:prop><Z:Copyright-Owner/></D:prop>
  </D:remove>
</D:propertyupdate>"#;

    #[test]
    fn parses_rfc_example_in_order() {
        let update = PropertyUpdate::from_xml(RFC_9_2_2.as_bytes().to_vec()).unwrap();
        assert_eq!(update.0.len(), 2);
        assert!(matches!(update.0[0], PropertyUpdateItem::Set(_)));
        assert!(matches!(update.0[1], PropertyUpdateItem::Remove(_)));
    }

    #[test]
    fn keeps_interleaved_order() {
        let xml = br#"<D:propertyupdate xmlns:D="DAV:"><D:remove><D:prop><D:a/></D:prop></D:remove><D:set><D:prop><D:b>1</D:b></D:prop></D:set><D:remove><D:prop><D:c/></D:prop></D:remove></D:propertyupdate>"#;
        let update = PropertyUpdate::from_xml(xml.to_vec()).unwrap();
        let kinds: Vec<_> = update
            .0
            .iter()
            .map(|item| matches!(item, PropertyUpdateItem::Set(_)))
            .collect();
        assert_eq!(kinds, [false, true, false]);
    }

    #[test]
    fn builder_writes_set_then_remove() {
        let update = PropertyUpdate::new()
            .set(Prop::builder().property(DisplayName("new".into())).build())
            .remove(Prop::builder().name::<DisplayName>().build());
        let xml = update.clone().into_xml().unwrap();
        let text = std::str::from_utf8(&xml).unwrap();
        assert!(text.contains("<D:set>"), "{text}");
        assert!(
            text.contains("<D:displayname>new</D:displayname>"),
            "{text}"
        );
        assert!(text.contains("<D:remove>"), "{text}");
        assert_eq!(PropertyUpdate::from_xml(xml).unwrap(), update);
    }

    #[test]
    fn empty_update_is_rejected() {
        let error = PropertyUpdate::from_xml(
            br#"<D:propertyupdate xmlns:D="DAV:"> </D:propertyupdate>"#.to_vec(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "propertyupdate",
                element: "set or remove",
            }
        ));
    }
}
