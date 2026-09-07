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
#[derive(Clone, Debug, Default)]
pub struct ValueMap {
    values: InnerValueMap,
    order: Vec<(ElementName<ByteString>, usize)>,
}

impl PartialEq for ValueMap {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl Eq for ValueMap {}

impl ValueMap {
    /// Create an empty value map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
        self.values
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
        self.values
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
        let tracks_order = self.values.is_empty() || !self.order.is_empty();
        let occurrence_count = match &value {
            Value::List(values) => values.len(),
            Value::Empty | Value::Text(_) | Value::Map(_) => 1,
        };
        self.values.insert(key.clone(), value);

        if tracks_order {
            self.order.retain(|(name, _)| name != &key);
            if let Some((key, _)) = self.values.get_key_value(&key) {
                self.order
                    .extend((0..occurrence_count).map(|index| (key.clone(), index)));
            }
        }
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

        match self.values.get(&E::element_name::<&'static str>()) {
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
        self.values.len()
    }

    /// Whether this map contains no child elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Iterate over child names and values in document order.
    pub fn iter(&self) -> impl Iterator<Item = (&ElementName<ByteString>, &Value)> {
        self.values.iter()
    }

    /// Iterate over children in their insertion order when it is available.
    ///
    /// Maps without insertion metadata fall back to their regular iteration
    /// order.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::{ElementName, Value, ValueMap};
    ///
    /// let mut map = ValueMap::new();
    /// map.insert_raw(
    ///     ElementName {
    ///         namespace: None,
    ///         prefix: None,
    ///         local_name: "example".into(),
    ///     },
    ///     Value::Empty,
    /// );
    /// assert_eq!(map.iter_ordered().count(), 1);
    /// ```
    pub fn iter_ordered(&self) -> impl Iterator<Item = (&ElementName<ByteString>, &Value)> {
        self.order
            .iter()
            .filter_map(|(order_key, index)| {
                self.values
                    .get_key_value(order_key)
                    .and_then(|(key, value)| match value {
                        Value::List(list) => list.get(*index).map(|value| (key, value)),
                        value if *index == 0 => Some((key, value)),
                        _ => None,
                    })
            })
            .chain(
                self.order
                    .is_empty()
                    .then_some(self.values.iter())
                    .into_iter()
                    .flatten(),
            )
    }

    /// Append a raw child value, grouping duplicate names into a list.
    pub fn insert_raw(&mut self, key: ElementName<ByteString>, value: Value) {
        if let Value::List(list) = value {
            let list = *list;
            std::iter::once(list.head)
                .chain(list.tail)
                .for_each(|value| self.insert_raw(key.clone(), value));
            return;
        }

        let tracks_order = self.values.is_empty() || !self.order.is_empty();
        let index = match self.values.get(&key) {
            Some(Value::List(values)) => values.len(),
            Some(_) => 1,
            None => 0,
        };

        let order_key = match self.values.entry(key) {
            indexmap::map::Entry::Occupied(mut entry) => {
                let order_key = tracks_order.then(|| entry.key().clone());
                match entry.get_mut() {
                    Value::List(list) => list.push(value),
                    old_value => {
                        let first = std::mem::take(old_value);
                        *old_value = Value::List(Box::new(nonempty![first, value]));
                    }
                }
                order_key
            }
            indexmap::map::Entry::Vacant(entry) => {
                let order_key = tracks_order.then(|| entry.key().clone());
                entry.insert(value);
                order_key
            }
        };

        if let Some(key) = order_key {
            self.order.push((key, index));
        }
    }
}

impl AsRef<InnerValueMap> for ValueMap {
    fn as_ref(&self) -> &InnerValueMap {
        &self.values
    }
}

impl AsMut<InnerValueMap> for ValueMap {
    fn as_mut(&mut self) -> &mut InnerValueMap {
        self.order.clear();
        &mut self.values
    }
}

impl IntoIterator for ValueMap {
    type Item = (ElementName<ByteString>, Value);
    type IntoIter = <InnerValueMap as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl From<InnerValueMap> for ValueMap {
    fn from(map: InnerValueMap) -> Self {
        Self {
            values: map,
            order: Vec::new(),
        }
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

    #[test]
    fn iter_ordered_preserves_interleaving() {
        let mut map = ValueMap::new();
        let alpha = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "alpha".into(),
        };
        let beta = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "beta".into(),
        };

        map.insert_raw(alpha.clone(), Value::Text("first".into()));
        map.insert_raw(beta, Value::Empty);
        map.insert_raw(alpha, Value::Text("second".into()));

        let children: Vec<_> = map
            .iter_ordered()
            .map(|(name, value)| (&*name.local_name, value.as_str().ok().map(|text| &**text)))
            .collect();
        assert_eq!(
            children,
            [
                ("alpha", Some("first")),
                ("beta", None),
                ("alpha", Some("second"))
            ]
        );
    }

    #[test]
    fn iter_ordered_falls_back_without_order_metadata() {
        let alpha = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "alpha".into(),
        };
        let beta = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "beta".into(),
        };
        let map = ValueMap::from(IndexMap::from([
            (alpha, Value::Text("first".into())),
            (beta, Value::Empty),
        ]));

        let names: Vec<_> = map
            .iter_ordered()
            .map(|(name, _)| &*name.local_name)
            .collect();
        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn as_mut_invalidates_order_metadata() {
        let alpha = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "alpha".into(),
        };
        let beta = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "beta".into(),
        };
        let mut map = ValueMap::new();
        map.insert_raw(alpha, Value::Empty);
        map.as_mut().insert(beta, Value::Empty);

        let names: Vec<_> = map
            .iter_ordered()
            .map(|(name, _)| &*name.local_name)
            .collect();
        assert_eq!(names, ["alpha", "beta"]);
    }

    #[test]
    fn insert_raw_flattens_incoming_lists_and_tracks_each_occurrence() {
        let alpha = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "alpha".into(),
        };
        let beta = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "beta".into(),
        };
        let mut map = ValueMap::new();
        map.insert_raw(
            alpha.clone(),
            Value::List(Box::new(nonempty::nonempty![
                Value::Text("first".into()),
                Value::Text("second".into()),
            ])),
        );
        map.insert_raw(beta, Value::Empty);
        map.insert_raw(
            alpha.clone(),
            Value::List(Box::new(nonempty::nonempty![
                Value::Text("third".into()),
                Value::Text("fourth".into()),
            ])),
        );

        let children: Vec<_> = map
            .iter_ordered()
            .map(|(name, value)| {
                (
                    name.local_name.to_string(),
                    value.as_str().map(ToString::to_string).ok(),
                )
            })
            .collect();
        assert_eq!(
            children,
            [
                ("alpha".to_owned(), Some("first".to_owned())),
                ("alpha".to_owned(), Some("second".to_owned())),
                ("beta".to_owned(), None),
                ("alpha".to_owned(), Some("third".to_owned())),
                ("alpha".to_owned(), Some("fourth".to_owned())),
            ]
        );
        let values = map.as_ref().get(&alpha).unwrap().as_list().unwrap();
        assert_eq!(values.len(), 4);
        assert!(values.iter().all(|value| !value.is_list()));
    }

    #[test]
    fn equality_ignores_child_insertion_order() {
        let alpha = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "alpha".into(),
        };
        let beta = ElementName {
            namespace: Some("DAV:".into()),
            prefix: None,
            local_name: "beta".into(),
        };
        let mut first = ValueMap::new();
        first.insert_raw(alpha.clone(), Value::Text("first".into()));
        first.insert_raw(beta.clone(), Value::Empty);
        first.insert_raw(alpha.clone(), Value::Text("second".into()));

        let mut second = ValueMap::new();
        second.insert_raw(alpha.clone(), Value::Text("first".into()));
        second.insert_raw(alpha, Value::Text("second".into()));
        second.insert_raw(beta, Value::Empty);

        assert_eq!(first, second);
    }
}
