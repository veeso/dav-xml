// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bytestring::ByteString;
use indexmap::IndexMap;
use nonempty::{NonEmpty, nonempty};

use crate::Error;
use crate::element::{Element, ElementExt, ElementName};

/// Represents the content of an XML element.
///
/// This data structure is intended to be similar to
/// [`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html).
/// Unlike when deserializing JSON, which has explicit arrays, we have to
/// manually group multiple adjacent elements into an array-like structure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Value {
    /// The element is empty, e.g. `<foo />`
    #[default]
    Empty,
    /// The element contains a text node, e.g. `<foo>bar</foo>`
    Text(ByteString),
    /// The element contains other elements, e.g. `<foo><bar /></foo>`
    Map(ValueMap),
    /// The parent element contains multiple elements of this type, e.g. `<foo
    /// /><foo />`
    List(Box<NonEmpty<Value>>),
}

impl Value {
    /// Borrow text content from this value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValueType`] when this value is not text.
    pub fn as_str(&self) -> Result<&ByteString, Error> {
        self.as_str_of_name("value")
    }

    /// Borrow text content and identify the expected element in an error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValueType`] when this value is not text.
    pub fn as_str_of<E: Element>(&self) -> Result<&ByteString, Error> {
        self.as_str_of_name(E::LOCAL_NAME)
    }

    fn as_str_of_name(&self, element: &'static str) -> Result<&ByteString, Error> {
        match self {
            Self::Text(s) => Ok(s),
            _ => Err(Error::InvalidValueType {
                element,
                expected: "text",
            }),
        }
    }

    /// Borrow the child map from this value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValueType`] when this value is not a map.
    pub fn as_map(&self) -> Result<&ValueMap, Error> {
        self.as_map_of_name("value")
    }

    /// Borrow the child map and identify the expected element in an error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValueType`] when this value is not a map.
    pub fn as_map_of<E: Element>(&self) -> Result<&ValueMap, Error> {
        self.as_map_of_name(E::LOCAL_NAME)
    }

    fn as_map_of_name(&self, element: &'static str) -> Result<&ValueMap, Error> {
        match self {
            Self::Map(map) => Ok(map),
            _ => Err(Error::InvalidValueType {
                element,
                expected: "child elements",
            }),
        }
    }

    /// Borrow the repeated sibling list from this value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValueType`] when this value is not a list.
    pub fn as_list(&self) -> Result<&NonEmpty<Value>, Error> {
        self.as_list_of_name("value")
    }

    /// Borrow the repeated sibling list and identify the expected element in
    /// an error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidValueType`] when this value is not a list.
    pub fn as_list_of<E: Element>(&self) -> Result<&NonEmpty<Value>, Error> {
        self.as_list_of_name(E::LOCAL_NAME)
    }

    fn as_list_of_name(&self, element: &'static str) -> Result<&NonEmpty<Value>, Error> {
        match self {
            Self::List(list) => Ok(list),
            _ => Err(Error::InvalidValueType {
                element,
                expected: "repeated elements",
            }),
        }
    }

    /// Whether this value contains repeated siblings.
    #[must_use]
    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    /// Whether this value contains child elements.
    #[must_use]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s.into())
    }
}

/// Convert a sequence of elements into the corresponding XML value.
pub(crate) fn list_value<E>(items: Vec<E>) -> Value
where
    E: Element + Into<Value>,
{
    let mut values = items.into_iter().map(Into::into);
    let Some(first) = values.next() else {
        return Value::Empty;
    };
    let Some(second) = values.next() else {
        return first;
    };

    let mut list = NonEmpty::new(first);
    list.push(second);
    list.extend(values);
    Value::List(Box::new(list))
}

/// Convert a sequence of elements into a value map under their element name.
pub(crate) fn list_value_under<E>(items: Vec<E>) -> Value
where
    E: Element + Into<Value>,
{
    let value = list_value(items);
    if value == Value::Empty {
        return Value::Empty;
    }

    let mut map = ValueMap::new();
    map.insert::<E>(value);
    Value::Map(map)
}

type InnerValueMap = IndexMap<ElementName<ByteString>, Value>;

/// A mapping from tag names to [`Value`]s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValueMap(pub(crate) InnerValueMap);

impl ValueMap {
    /// Create an empty value map.
    #[must_use]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    /// Extract a child element of a specific type.
    ///
    /// # Returns
    ///
    /// - `None` if the element doesn't exist
    /// - `Some(Ok(_))` if the element exists and was successfully extracted
    /// - `Some(Err(_))` if the element exists and extraction failed
    #[must_use]
    pub fn get<'v, E>(&'v self) -> Option<Result<E, Error>>
    where
        E: Element + TryFrom<&'v Value, Error = Error>,
    {
        self.0
            .get(&E::element_name::<&'static str>())
            .map(E::try_from)
    }
    /// Extract a non-empty child element of a specific type.
    ///
    /// # Returns
    ///
    /// - `None` if the element doesn't exist
    /// - `Some(None)` if the element exists and is empty
    /// - `Some(Some(Ok(_)))` if the element exists, is not empty and was
    ///   successfully extracted
    /// - `Some(Some(Err(_)))` if the element exists, is not empty and
    ///   extraction failed
    #[must_use]
    pub fn get_optional<'v, E>(&'v self) -> Option<Option<Result<E, Error>>>
    where
        E: Element + TryFrom<&'v Value, Error = Error>,
    {
        self.0
            .get(&E::element_name::<&'static str>())
            .map(|value| match value {
                Value::Empty => None,
                v => Some(v.try_into()),
            })
    }

    /// Extract a required child element of type `E` from parent type `P`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingElement`] when the child is absent, or the
    /// child's conversion error when it cannot be parsed.
    pub fn get_required<'v, P, E>(&'v self) -> Result<E, Error>
    where
        P: Element,
        E: Element + TryFrom<&'v Value, Error = Error>,
    {
        self.get::<E>().ok_or(Error::MissingElement {
            parent: P::LOCAL_NAME,
            element: E::LOCAL_NAME,
        })?
    }

    /// Insert a child value into the map.
    pub fn insert<E: Element>(&mut self, value: Value) {
        let key = E::element_name();
        self.0.insert(key, value);
    }
}

impl ValueMap {
    pub(crate) fn iter_all<'v, E>(&'v self) -> impl Iterator<Item = Result<E, Error>> + 'v
    where
        E: Element + TryFrom<&'v Value, Error = Error> + 'v,
    {
        enum ElementIter<'a> {
            List(nonempty::Iter<'a, Value>),
            Single(std::iter::Once<&'a Value>),
            Empty,
        }

        impl<'a> Iterator for ElementIter<'a> {
            type Item = &'a Value;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::List(inner) => inner.next(),
                    Self::Single(value) => value.next(),
                    Self::Empty => None,
                }
            }
        }

        match self.0.get(&E::element_name::<&'static str>()) {
            Some(Value::List(list)) => ElementIter::List(list.iter()),
            Some(value) => ElementIter::Single(std::iter::once(value)),
            None => ElementIter::Empty,
        }
        .map(E::try_from)
    }

    /// Extract every sibling child element of type `E`, in document order.
    ///
    /// # Errors
    ///
    /// Returns the first child conversion error.
    pub fn get_all<'v, E>(&'v self) -> Result<Vec<E>, Error>
    where
        E: Element + TryFrom<&'v Value, Error = Error> + 'v,
    {
        self.iter_all::<E>().collect()
    }

    /// Return the number of distinct child element names in this map.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this map contains no child elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over child names and values in document order.
    pub fn iter(&self) -> impl Iterator<Item = (&ElementName<ByteString>, &Value)> {
        self.0.iter()
    }

    /// Append a raw child value, grouping duplicate names into a list.
    pub fn insert_raw(&mut self, key: ElementName<ByteString>, value: Value) {
        match self.0.get_mut(&key) {
            Some(Value::List(list)) => list.push(value),
            Some(old_value) => {
                let first = std::mem::take(old_value);
                *old_value = Value::List(Box::new(nonempty![first, value]));
            }
            None => {
                self.0.insert(key, value);
            }
        }
    }
}

impl AsRef<InnerValueMap> for ValueMap {
    fn as_ref(&self) -> &InnerValueMap {
        &self.0
    }
}

impl AsMut<InnerValueMap> for ValueMap {
    fn as_mut(&mut self) -> &mut InnerValueMap {
        &mut self.0
    }
}

impl IntoIterator for ValueMap {
    type Item = (ElementName<ByteString>, Value);
    type IntoIter = <InnerValueMap as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<InnerValueMap> for ValueMap {
    fn from(map: InnerValueMap) -> Self {
        Self(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::elements::Href;

    fn href_key() -> ElementName<ByteString> {
        ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "href".into(),
        }
    }

    #[test]
    fn insert_raw_keeps_single_value() {
        let mut map = ValueMap::new();
        map.insert_raw(href_key(), Value::Text("/a".into()));
        assert_eq!(map.get::<Href>().unwrap().unwrap().path(), "/a");
    }

    #[test]
    fn insert_raw_flattens_three_siblings() {
        let mut map = ValueMap::new();
        for path in ["/a", "/b", "/c"] {
            map.insert_raw(href_key(), Value::Text(path.into()));
        }
        let hrefs = map.get_all::<Href>().unwrap();
        let paths: Vec<_> = hrefs.iter().map(|href| href.path().to_string()).collect();
        assert_eq!(paths, ["/a", "/b", "/c"]);
        let list = map.as_ref().get(&href_key()).unwrap().as_list().unwrap();
        assert!(list.iter().all(|v| !v.is_list()), "no nested lists");
    }

    #[test]
    fn get_optional_distinguishes_empty() {
        let mut map = ValueMap::new();
        map.insert_raw(href_key(), Value::Empty);
        assert!(matches!(map.get_optional::<Href>(), Some(None)));
    }

    #[test]
    fn get_all_on_absent_is_empty_vec() {
        let map = ValueMap::new();
        assert!(map.get_all::<Href>().unwrap().is_empty());
    }

    #[test]
    fn lookup_ignores_prefix() {
        let mut map = ValueMap::new();
        map.insert_raw(
            ElementName {
                namespace: Some("DAV:".into()),
                prefix: Some("lp1".into()),
                local_name: "href".into(),
            },
            Value::Text("/x".into()),
        );
        assert!(map.get::<Href>().is_some());
    }

    #[test]
    fn insert_replaces_existing() {
        let mut map = ValueMap::new();
        map.insert::<Href>(Value::Text("/a".into()));
        map.insert::<Href>(Value::Text("/b".into()));
        assert_eq!(map.get_all::<Href>().unwrap().len(), 1);
        assert_eq!(map.get::<Href>().unwrap().unwrap().path(), "/b");
    }
}
