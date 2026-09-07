// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;

use bytestring::ByteString;
use quick_xml::escape::{resolve_predefined_entity, unescape};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;

use crate::element::ElementName;
use crate::utils::BytesExt;
use crate::value::{ContentItem, ValueMap};
use crate::{Error, Result, Value};

pub(crate) fn read_xml(xml: impl Into<bytes::Bytes>) -> Result<Value> {
    let xml = xml.into();
    let mut reader = XmlReader::new(std::str::from_utf8(&xml)?);
    reader.read_into_value(&xml, ReadMode::Structured)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReadMode {
    Structured,
    Prop,
    Property,
    Mixed,
    Error,
}

#[derive(Default)]
struct TextRun(Option<String>);

impl TextRun {
    fn push(&mut self, text: &str) {
        self.0.get_or_insert_default().push_str(text);
    }

    fn take(&mut self) -> Option<ByteString> {
        self.0.take().map(Into::into)
    }

    fn flush(&mut self, content: &mut Vec<ContentItem>) {
        if let Some(text) = self.take() {
            content.push(ContentItem::Text(text));
        }
    }
}

struct XmlReader<'x> {
    reader: quick_xml::NsReader<&'x [u8]>,
    last: Option<quick_xml::events::Event<'x>>,
}

impl<'x> XmlReader<'x> {
    fn new(xml: &'x str) -> Self {
        Self {
            reader: quick_xml::NsReader::from_str(xml),
            last: None,
        }
    }
    fn last(&self) -> Option<&quick_xml::events::Event<'x>> {
        self.last.as_ref()
    }
    fn read_resolved_event(
        &mut self,
    ) -> quick_xml::Result<(
        quick_xml::name::ResolveResult<'_>,
        quick_xml::events::Event<'_>,
    )> {
        let (resolve_result, event) = self.reader.read_resolved_event()?;
        self.last = Some(event.clone());
        Ok((resolve_result, event))
    }

    fn read_into_value(&mut self, xml: &bytes::Bytes, mode: ReadMode) -> Result<Value> {
        let mut map = ValueMap::new();
        let mut content = Vec::new();
        let mut pending_text = TextRun::default();

        loop {
            let (resolve_result, event) = self.read_resolved_event()?;
            match event {
                Event::Text(text) => {
                    if matches!(
                        mode,
                        ReadMode::Structured | ReadMode::Prop | ReadMode::Error
                    ) && text.chars().all(char::is_whitespace)
                    {
                        continue;
                    }

                    let text = unescape(&text).map_err(quick_xml::Error::from)?;
                    if matches!(mode, ReadMode::Property | ReadMode::Mixed) {
                        pending_text.push(&text);
                        continue;
                    }

                    let head: ByteString = xml.maybe_slice_ref(text.as_bytes()).try_into()?;
                    drop(text);

                    return Ok(Value::Text(self.read_trailing_text(head)?));
                }
                Event::Start(start) => {
                    pending_text.flush(&mut content);
                    validate_namespace_declarations(xml, &start)?;
                    let key = key(xml, &resolve_result, &start)?;
                    let start_name = xml.maybe_slice_ref(start.name().as_ref().as_bytes());
                    drop(resolve_result);
                    drop(start);

                    let child_value = self.read_into_value(xml, child_mode(mode, &key))?;

                    if matches!(mode, ReadMode::Property | ReadMode::Mixed) {
                        content.push(ContentItem::Element {
                            name: key,
                            value: child_value,
                        });
                    } else {
                        map.insert_raw(key, child_value);
                    }

                    if !matches!(self.last(), Some(Event::End(end)) if start_name == end.name().as_ref().as_bytes())
                    {
                        return Err(Error::UnexpectedTag {
                            position: self.reader.buffer_position(),
                        });
                    }
                }
                Event::Empty(tag) => {
                    pending_text.flush(&mut content);
                    validate_namespace_declarations(xml, &tag)?;
                    let key = key(xml, &resolve_result, &tag)?;
                    if matches!(mode, ReadMode::Property | ReadMode::Mixed) {
                        content.push(ContentItem::Element {
                            name: key,
                            value: Value::Empty,
                        });
                    } else {
                        map.insert_raw(key, Value::Empty);
                    }
                }
                Event::End(_) | Event::Eof => break,
                Event::Comment(_) | Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {}
                Event::CData(cdata) => {
                    let head: ByteString = cdata.as_ref().into();
                    drop(cdata);

                    if matches!(mode, ReadMode::Property | ReadMode::Mixed) {
                        pending_text.push(&head);
                        continue;
                    }

                    return Ok(Value::Text(self.read_trailing_text(head)?));
                }
                Event::GeneralRef(reference) => {
                    let head: ByteString = match reference.resolve_char_ref()? {
                        Some(c) => c.to_string().into(),
                        None => resolve_predefined_entity(&reference)
                            .ok_or_else(|| Error::UnknownEntity(reference.to_string()))?
                            .into(),
                    };
                    drop(reference);

                    if matches!(mode, ReadMode::Property | ReadMode::Mixed) {
                        pending_text.push(&head);
                        continue;
                    }

                    return Ok(Value::Text(self.read_trailing_text(head)?));
                }
            }
        }

        pending_text.flush(&mut content);

        Ok(match mode {
            ReadMode::Mixed => mixed_value(content),
            ReadMode::Property => property_value(content),
            ReadMode::Structured | ReadMode::Prop | ReadMode::Error => Value::Map(map),
        })
    }

    /// Consumes the remaining character data of the current element and appends
    /// it to `head`.
    ///
    /// `quick-xml` splits character data at every entity or character reference
    /// and at every CDATA section, so a single text node can arrive as several
    /// events. The common case is a single [`Event::Text`], which is returned
    /// without copying.
    fn read_trailing_text(&mut self, head: ByteString) -> Result<ByteString> {
        let mut tail: Option<String> = None;

        loop {
            let (_, event) = self.read_resolved_event()?;
            let chunk: Cow<'_, str> = match event {
                quick_xml::events::Event::Text(ref text) => {
                    unescape(text).map_err(quick_xml::Error::from)?
                }
                quick_xml::events::Event::CData(ref cdata) => Cow::Borrowed(cdata),
                quick_xml::events::Event::GeneralRef(ref reference) => {
                    match reference.resolve_char_ref()? {
                        Some(c) => Cow::Owned(c.to_string()),
                        None => Cow::Borrowed(
                            resolve_predefined_entity(reference)
                                .ok_or_else(|| Error::UnknownEntity(reference.to_string()))?,
                        ),
                    }
                }
                quick_xml::events::Event::Comment(_) | quick_xml::events::Event::PI(_) => {
                    Cow::Borrowed("")
                }
                quick_xml::events::Event::End(_) | quick_xml::events::Event::Eof => break,
                _ => {
                    return Err(Error::UnexpectedTag {
                        position: self.reader.buffer_position(),
                    });
                }
            };
            tail.get_or_insert_with(String::new).push_str(&chunk);
        }

        Ok(match tail {
            None => head,
            Some(tail) => ByteString::from(format!("{head}{tail}")),
        })
    }
}

fn key(
    xml: &bytes::Bytes,
    resolve_result: &ResolveResult,
    tag: &BytesStart<'_>,
) -> Result<ElementName<ByteString>> {
    match resolve_result {
        ResolveResult::Bound(ns) => {
            if ns.as_ref().is_empty() {
                return Err(Error::InvalidNamespace(
                    xml.maybe_slice_ref(ns.as_ref().as_bytes()),
                ));
            }

            Ok(ElementName {
                namespace: Some(xml.maybe_slice_ref(ns.as_ref().as_bytes()).try_into()?),
                prefix: None,
                local_name: xml
                    .maybe_slice_ref(tag.local_name().as_ref().as_bytes())
                    .try_into()?,
            })
        }

        ResolveResult::Unbound | ResolveResult::Unknown(_) => Ok(ElementName {
            namespace: None,
            prefix: None,
            local_name: xml
                .maybe_slice_ref(tag.name().as_ref().as_bytes())
                .try_into()?,
        }),
    }
}

fn validate_namespace_declarations(xml: &bytes::Bytes, tag: &BytesStart<'_>) -> Result<()> {
    for attribute in tag.attributes().with_checks(false) {
        let attribute = attribute.map_err(quick_xml::Error::from)?;
        if attribute.key.as_ref().starts_with("xmlns:") && attribute.value.as_ref().is_empty() {
            return Err(Error::InvalidNamespace(
                xml.maybe_slice_ref(attribute.value.as_ref().as_bytes()),
            ));
        }
    }
    Ok(())
}

fn is_owner_name(name: &ElementName<ByteString>) -> bool {
    name.namespace.as_deref() == Some(crate::DAV_NAMESPACE) && name.local_name == "owner"
}

fn is_prop_name(name: &ElementName<ByteString>) -> bool {
    name.namespace.as_deref() == Some(crate::DAV_NAMESPACE) && name.local_name == "prop"
}

fn is_error_name(name: &ElementName<ByteString>) -> bool {
    name.namespace.as_deref() == Some(crate::DAV_NAMESPACE) && name.local_name == "error"
}

fn is_known_condition_name(name: &ElementName<ByteString>) -> bool {
    name.namespace.as_deref() == Some(crate::DAV_NAMESPACE)
        && matches!(
            name.local_name.as_ref(),
            "lock-token-matches-request-uri"
                | "lock-token-submitted"
                | "no-conflicting-lock"
                | "no-external-entities"
                | "preserved-live-properties"
                | "propfind-finite-depth"
                | "cannot-modify-protected-property"
        )
}

fn is_structured_property_name(name: &ElementName<ByteString>) -> bool {
    name.namespace.as_deref() == Some(crate::DAV_NAMESPACE)
        && matches!(
            name.local_name.as_ref(),
            "lockdiscovery" | "resourcetype" | "supportedlock"
        )
}

fn child_mode(mode: ReadMode, name: &ElementName<ByteString>) -> ReadMode {
    match mode {
        ReadMode::Mixed => ReadMode::Mixed,
        ReadMode::Prop => {
            if is_owner_name(name) {
                ReadMode::Mixed
            } else if is_structured_property_name(name) {
                ReadMode::Structured
            } else {
                ReadMode::Property
            }
        }
        ReadMode::Property => {
            if is_owner_name(name) {
                ReadMode::Mixed
            } else {
                ReadMode::Property
            }
        }
        ReadMode::Error => {
            if is_known_condition_name(name) {
                ReadMode::Structured
            } else {
                ReadMode::Property
            }
        }
        ReadMode::Structured => {
            if is_owner_name(name) {
                ReadMode::Mixed
            } else if is_error_name(name) {
                ReadMode::Error
            } else if is_prop_name(name) {
                ReadMode::Prop
            } else {
                ReadMode::Structured
            }
        }
    }
}

fn property_value(items: Vec<ContentItem>) -> Value {
    let has_element = items
        .iter()
        .any(|item| matches!(item, ContentItem::Element { .. }));
    let first_significant_text = items.iter().position(|item| {
        matches!(item, ContentItem::Text(text) if text.chars().any(|character| !character.is_whitespace()))
    });

    if !has_element {
        return if items.is_empty() {
            Value::Map(ValueMap::new())
        } else {
            Value::Text(
                items
                    .into_iter()
                    .filter_map(|item| match item {
                        ContentItem::Text(text) => Some(text.to_string()),
                        ContentItem::Element { .. } => None,
                    })
                    .collect::<String>()
                    .into(),
            )
        };
    }

    if first_significant_text.is_some() {
        return Value::Mixed(items);
    }

    if items
        .iter()
        .any(|item| matches!(item, ContentItem::Text(_)))
    {
        Value::Mixed(items)
    } else {
        mixed_value(
            items
                .into_iter()
                .filter(|item| matches!(item, ContentItem::Element { .. }))
                .collect(),
        )
    }
}

fn mixed_value(items: Vec<ContentItem>) -> Value {
    let has_text = items
        .iter()
        .any(|item| matches!(item, ContentItem::Text(_)));
    let has_element = items
        .iter()
        .any(|item| matches!(item, ContentItem::Element { .. }));

    match (has_text, has_element) {
        (false, false) => Value::Empty,
        (true, false) => Value::Text(
            items
                .into_iter()
                .filter_map(|item| match item {
                    ContentItem::Text(text) => Some(text.to_string()),
                    ContentItem::Element { .. } => None,
                })
                .collect::<String>()
                .into(),
        ),
        (false, true) => {
            let mut map = ValueMap::new();
            items
                .into_iter()
                .filter_map(|item| match item {
                    ContentItem::Element { name, value } => Some((name, value)),
                    ContentItem::Text(_) => None,
                })
                .for_each(|(name, value)| map.insert_raw(name, value));
            Value::Map(map)
        }
        (true, true) => Value::Mixed(items),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn key(namespace: Option<&str>, local_name: &str) -> ElementName<ByteString> {
        ElementName {
            namespace: namespace.map(Into::into),
            prefix: None,
            local_name: local_name.into(),
        }
    }

    fn root(xml: &str) -> ValueMap {
        let value = read_xml(xml.as_bytes().to_vec()).unwrap();
        let Value::Map(map) = value else {
            panic!("root is a map")
        };
        map
    }

    #[test]
    fn reads_empty_text_and_nested_elements() {
        let map = root(r#"<a xmlns="DAV:"><b/><c>text</c><d><e/></d></a>"#);
        let a = map
            .as_ref()
            .get(&key(Some("DAV:"), "a"))
            .unwrap()
            .as_map()
            .unwrap();
        assert_eq!(a.as_ref().get(&key(Some("DAV:"), "b")), Some(&Value::Empty));
        assert_eq!(
            a.as_ref().get(&key(Some("DAV:"), "c")),
            Some(&Value::Text("text".into()))
        );
        assert!(a.as_ref().get(&key(Some("DAV:"), "d")).unwrap().is_map());
    }

    #[test]
    fn same_namespace_different_prefixes_share_key() {
        let map =
            root(r#"<D:a xmlns:D="DAV:" xmlns:lp1="DAV:"><lp1:x>1</lp1:x><D:x>2</D:x></D:a>"#);
        let a = map
            .as_ref()
            .get(&key(Some("DAV:"), "a"))
            .unwrap()
            .as_map()
            .unwrap();
        let x = a.as_ref().get(&key(Some("DAV:"), "x")).unwrap();
        assert!(x.is_list());
    }

    #[test]
    fn undeclared_prefix_keeps_qualified_name() {
        let map = root(r#"<a xmlns="DAV:"><foo:bar/></a>"#);
        let a = map
            .as_ref()
            .get(&key(Some("DAV:"), "a"))
            .unwrap()
            .as_map()
            .unwrap();
        assert!(a.as_ref().contains_key(&key(None, "foo:bar")));
    }

    #[test]
    fn empty_namespace_uri_is_rejected() {
        let error = read_xml(br#"<x:a xmlns:x=""/>"#.to_vec()).unwrap_err();
        assert!(matches!(error, Error::InvalidNamespace(_)));
    }

    #[test]
    fn joins_text_split_by_entities_and_cdata() {
        let map = root("<a>x &amp; y &#38; <![CDATA[<z>]]> w</a>");
        assert_eq!(
            map.as_ref().get(&key(None, "a")),
            Some(&Value::Text("x & y & <z> w".into()))
        );
    }

    #[test]
    fn unknown_entity_is_an_error() {
        let error = read_xml(b"<a>&nope;</a>".to_vec()).unwrap_err();
        assert!(matches!(error, Error::UnknownEntity(name) if name == "nope"));
    }

    #[test]
    fn mismatched_closing_tag_is_an_error() {
        let error = read_xml(b"<a><b></a></b>".to_vec()).unwrap_err();
        assert!(matches!(error, Error::UnexpectedTag { .. } | Error::Xml(_)));
    }

    #[test]
    fn invalid_utf8_is_an_error() {
        let error = read_xml(vec![0xff, 0xfe, b'<']).unwrap_err();
        assert!(matches!(error, Error::Utf8(_)));
    }

    #[test]
    fn comments_pis_and_doctype_are_skipped() {
        let map = root("<?xml version=\"1.0\"?><!DOCTYPE a><!-- c --><a><?pi x?><b/></a>");
        assert!(map.as_ref().contains_key(&key(None, "a")));
    }

    #[test]
    fn whitespace_only_text_is_ignored() {
        let map = root("<a>\n  <b/>\n</a>");
        assert!(map.as_ref().contains_key(&key(None, "a")));
    }

    #[test]
    fn leading_cdata_is_text() {
        let map = root("<a><![CDATA[hi]]></a>");
        assert_eq!(
            map.as_ref().get(&key(None, "a")),
            Some(&Value::Text("hi".into()))
        );
    }

    #[test]
    fn accumulates_adjacent_text_chunks_in_one_pending_run() {
        let mut run = TextRun::default();
        for _ in 0..4096 {
            run.push("&");
        }

        assert_eq!(run.take(), Some("&".repeat(4096).into()));
        assert_eq!(run.take(), None);
    }
}
