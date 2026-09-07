// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use bytestring::ByteString;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

use crate::element::{Element, ElementExt, ElementName};
use crate::elements::DavError;
use crate::{ContentItem, DAV_NAMESPACE, Error, Result, Value};

pub(crate) fn write_xml<E: Element>(writer: impl std::io::Write, value: Value) -> Result<()> {
    let name = E::element_name();
    validate_value(&name, &value)?;

    let mut writer = XmlWriter {
        inner: quick_xml::Writer::new_with_indent(writer, b' ', 2),
        namespaces: BTreeMap::new(),
    };
    writer
        .inner
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;

    writer.collect_namespaces(&name, &value);
    writer.write_root::<E>(&name, value)
}

fn validate_value(name: &ElementName<ByteString>, value: &Value) -> Result<()> {
    if is_owner_name(name) {
        return Ok(());
    }

    match value {
        Value::List(list) => list.iter().try_for_each(|item| validate_value(name, item)),
        Value::Text(_) | Value::Empty => validate_error(name, value),
        Value::Map(map) => {
            validate_error(name, value)?;
            map.iter()
                .try_for_each(|(child, value)| validate_value(child, value))
        }
        Value::Mixed(items) => {
            validate_error(name, value)?;
            items.iter().try_for_each(validate_content_item)
        }
    }
}

fn validate_content_item(item: &ContentItem) -> Result<()> {
    match item {
        ContentItem::Text(_) => Ok(()),
        ContentItem::Element { name, value } => validate_value(name, value),
    }
}

fn validate_error(name: &ElementName<ByteString>, value: &Value) -> Result<()> {
    if name.namespace.as_deref() == Some(DAV_NAMESPACE) && name.local_name == DavError::LOCAL_NAME {
        DavError::try_from(value)?.validate()?;
    }
    Ok(())
}

struct XmlWriter<W: std::io::Write> {
    inner: quick_xml::Writer<W>,
    /// Namespace URI to prefix, sorted by URI for deterministic output.
    namespaces: BTreeMap<ByteString, ByteString>,
}

impl<W: std::io::Write> std::fmt::Debug for XmlWriter<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XmlWriter")
            .field("namespaces", &self.namespaces)
            .finish_non_exhaustive()
    }
}

impl<W: std::io::Write> XmlWriter<W> {
    fn add_namespace(&mut self, name: &ElementName<ByteString>) {
        let Some(namespace) = &name.namespace else {
            return;
        };
        if self.namespaces.contains_key(namespace) {
            return;
        }

        let taken: BTreeSet<&ByteString> = self.namespaces.values().collect();
        let prefix = match &name.prefix {
            Some(prefix) if !taken.contains(prefix) => prefix.clone(),
            _ => {
                let mut index = 0_usize;
                loop {
                    let candidate = ByteString::from(format!("ns{index}"));
                    if !taken.contains(&candidate) {
                        break candidate;
                    }
                    index += 1;
                }
            }
        };
        self.namespaces.insert(namespace.clone(), prefix);
    }

    fn collect_namespaces(&mut self, name: &ElementName<ByteString>, value: &Value) {
        self.add_namespace(name);
        match value {
            Value::Text(_) | Value::Empty => {}
            Value::List(list) => {
                for item in list.iter() {
                    self.collect_namespaces(name, item);
                }
            }
            Value::Map(map) => {
                for (child, value) in map.iter() {
                    self.collect_namespaces(child, value);
                }
            }
            Value::Mixed(items) => {
                for item in items {
                    if let ContentItem::Element { name, value } = item {
                        self.collect_namespaces(name, value);
                    }
                }
            }
        }
    }

    fn qualified<'n>(&self, name: &'n ElementName<ByteString>) -> Cow<'n, str> {
        match name
            .namespace
            .as_ref()
            .and_then(|ns| self.namespaces.get(ns))
        {
            Some(prefix) => Cow::Owned(format!(
                "{prefix}:{local_name}",
                local_name = name.local_name
            )),
            None => Cow::Borrowed(&name.local_name),
        }
    }

    fn root_start(&self, raw_name: &str) -> BytesStart<'static> {
        let mut start = BytesStart::new(raw_name.to_owned());
        for (namespace, prefix) in &self.namespaces {
            start.push_attribute(Attribute::from((
                format!("xmlns:{prefix}").as_str(),
                &**namespace,
            )));
        }
        start
    }

    fn write_root<E: Element>(
        &mut self,
        name: &ElementName<ByteString>,
        value: Value,
    ) -> Result<()> {
        let raw_name = self.qualified(name).into_owned();
        let start = self.root_start(&raw_name);
        match value {
            Value::Empty => self.inner.write_event(Event::Empty(start))?,
            Value::Text(text) => {
                self.inner.write_event(Event::Start(start))?;
                self.inner.write_event(Event::Text(BytesText::new(&text)))?;
                self.inner
                    .write_event(Event::End(BytesEnd::new(raw_name)))?;
            }
            Value::Map(map) => {
                self.inner.write_event(Event::Start(start))?;
                if is_owner_name(name) {
                    for (child, value) in map.iter_ordered() {
                        self.write_value_without_indent(child, value)?;
                    }
                    self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
                } else {
                    for (child, value) in map.iter_ordered() {
                        self.write_value(child, value)?;
                    }
                    self.inner
                        .write_event(Event::End(BytesEnd::new(raw_name)))?;
                }
            }
            Value::Mixed(items) => {
                self.inner.write_event(Event::Start(start))?;
                if is_owner_name(name) {
                    self.write_mixed_without_indent(&items)?;
                    self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
                } else {
                    self.write_mixed(&items)?;
                    self.inner
                        .write_event(Event::End(BytesEnd::new(raw_name)))?;
                }
            }
            Value::List(_) => {
                return Err(Error::InvalidValueType {
                    element: E::LOCAL_NAME,
                    expected: "a single root element",
                });
            }
        }
        Ok(())
    }

    fn write_value(&mut self, name: &ElementName<ByteString>, value: &Value) -> Result<()> {
        if is_owner_name(name) {
            return self.write_owner_value(name, value);
        }

        let raw_name = self.qualified(name).into_owned();
        match value {
            Value::Empty => self
                .inner
                .write_event(Event::Empty(BytesStart::new(raw_name)))?,
            Value::Text(text) => {
                self.inner
                    .write_event(Event::Start(BytesStart::new(raw_name.as_str())))?;
                self.inner.write_event(Event::Text(BytesText::new(text)))?;
                self.inner
                    .write_event(Event::End(BytesEnd::new(raw_name)))?;
            }
            Value::List(list) => {
                for item in list.iter() {
                    self.write_value(name, item)?;
                }
            }
            Value::Map(map) => {
                self.inner
                    .write_event(Event::Start(BytesStart::new(raw_name.as_str())))?;
                for (child, value) in map.iter_ordered() {
                    self.write_value(child, value)?;
                }
                self.inner
                    .write_event(Event::End(BytesEnd::new(raw_name)))?;
            }
            Value::Mixed(items) => {
                self.inner
                    .write_event(Event::Start(BytesStart::new(raw_name.as_str())))?;
                self.write_mixed(items)?;
                self.inner
                    .write_event(Event::End(BytesEnd::new(raw_name)))?;
            }
        }
        Ok(())
    }

    fn write_owner_value(&mut self, name: &ElementName<ByteString>, value: &Value) -> Result<()> {
        let raw_name = self.qualified(name).into_owned();
        match value {
            Value::Empty => self
                .inner
                .write_event(Event::Empty(BytesStart::new(raw_name)))?,
            Value::Text(text) => {
                self.inner
                    .write_event(Event::Start(BytesStart::new(raw_name.as_str())))?;
                self.inner.write_event(Event::Text(BytesText::new(text)))?;
                self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
            }
            Value::List(list) => {
                for item in list.iter() {
                    self.write_owner_value(name, item)?;
                }
            }
            Value::Map(map) => {
                self.inner
                    .write_event(Event::Start(BytesStart::new(raw_name.as_str())))?;
                for (child, value) in map.iter_ordered() {
                    self.write_value_without_indent(child, value)?;
                }
                self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
            }
            Value::Mixed(items) => {
                self.inner
                    .write_event(Event::Start(BytesStart::new(raw_name.as_str())))?;
                self.write_mixed_without_indent(items)?;
                self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
            }
        }
        Ok(())
    }

    fn write_value_without_indent(
        &mut self,
        name: &ElementName<ByteString>,
        value: &Value,
    ) -> Result<()> {
        let raw_name = self.qualified(name).into_owned();
        match value {
            Value::Empty => {
                self.write_event_without_indent(Event::Empty(BytesStart::new(raw_name)))?;
            }
            Value::Text(text) => {
                self.write_event_without_indent(Event::Start(BytesStart::new(raw_name.as_str())))?;
                self.inner.write_event(Event::Text(BytesText::new(text)))?;
                self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
            }
            Value::List(list) => {
                for item in list.iter() {
                    self.write_value_without_indent(name, item)?;
                }
            }
            Value::Map(map) => {
                self.write_event_without_indent(Event::Start(BytesStart::new(raw_name.as_str())))?;
                for (child, value) in map.iter_ordered() {
                    self.write_value_without_indent(child, value)?;
                }
                self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
            }
            Value::Mixed(items) => {
                self.write_event_without_indent(Event::Start(BytesStart::new(raw_name.as_str())))?;
                self.write_mixed_without_indent(items)?;
                self.write_event_without_indent(Event::End(BytesEnd::new(raw_name)))?;
            }
        }
        Ok(())
    }

    fn write_event_without_indent(&mut self, event: Event<'_>) -> Result<()> {
        self.inner.write_event(Event::Text(BytesText::new("")))?;
        self.inner.write_event(event)?;
        Ok(())
    }

    fn write_mixed(&mut self, items: &[ContentItem]) -> Result<()> {
        for item in items {
            match item {
                ContentItem::Text(text) => {
                    self.inner.write_event(Event::Text(BytesText::new(text)))?;
                }
                ContentItem::Element { name, value } => {
                    self.write_value_without_indent(name, value)?;
                }
            }
        }
        Ok(())
    }

    fn write_mixed_without_indent(&mut self, items: &[ContentItem]) -> Result<()> {
        for item in items {
            match item {
                ContentItem::Text(text) => {
                    self.inner.write_event(Event::Text(BytesText::new(text)))?;
                }
                ContentItem::Element { name, value } => {
                    self.write_value_without_indent(name, value)?;
                }
            }
        }
        Ok(())
    }
}

fn is_owner_name(name: &ElementName<ByteString>) -> bool {
    name.namespace.as_deref() == Some(DAV_NAMESPACE) && name.local_name == "owner"
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::DAV_PREFIX;
    use crate::elements::{Condition, LockInfo, Owner};
    use crate::value::ValueMap;

    struct Root;

    impl Element for Root {
        const NAMESPACE: &'static str = DAV_NAMESPACE;
        const PREFIX: &'static str = DAV_PREFIX;
        const LOCAL_NAME: &'static str = "root";
    }

    struct Alpha;

    impl Element for Alpha {
        const NAMESPACE: &'static str = DAV_NAMESPACE;
        const PREFIX: &'static str = DAV_PREFIX;
        const LOCAL_NAME: &'static str = "alpha";
    }

    fn name(ns: &str, prefix: &str, local: &str) -> ElementName<ByteString> {
        ElementName {
            namespace: Some(ns.into()),
            prefix: Some(prefix.into()),
            local_name: local.into(),
        }
    }

    fn render(value: Value) -> String {
        let mut out = Vec::new();
        write_xml::<Root>(&mut out, value).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn declares_namespaces_in_sorted_order() {
        let mut map = ValueMap::new();
        map.insert_raw(name("urn:z", "z", "a"), Value::Empty);
        map.insert_raw(name("urn:a", "a", "b"), Value::Empty);
        let xml = render(Value::Map(map));
        assert_eq!(
            xml,
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
             <D:root xmlns:D=\"DAV:\" xmlns:a=\"urn:a\" xmlns:z=\"urn:z\">\n  \
             <z:a/>\n  <a:b/>\n</D:root>"
        );
    }

    #[test]
    fn serializes_repeated_error_siblings() {
        let mut map = ValueMap::new();
        let error_name = name(DAV_NAMESPACE, DAV_PREFIX, DavError::LOCAL_NAME);
        map.insert_raw(
            error_name.clone(),
            DavError::single(Condition::PropfindFiniteDepth).into(),
        );
        map.insert_raw(
            error_name,
            DavError::single(Condition::NoExternalEntities).into(),
        );

        let xml = render(Value::Map(map));
        assert_eq!(xml.matches("<D:error").count(), 2);
    }

    #[test]
    fn writes_interleaved_siblings_in_order() {
        let mut map = ValueMap::new();
        let alpha = name(DAV_NAMESPACE, DAV_PREFIX, "alpha");
        map.insert_raw(alpha.clone(), Value::Text("first".into()));
        map.insert_raw(name(DAV_NAMESPACE, DAV_PREFIX, "beta"), Value::Empty);
        map.insert_raw(alpha, Value::Text("second".into()));

        let xml = render(Value::Map(map));
        let alpha_first = xml.find("<D:alpha>first</D:alpha>").unwrap();
        let beta = xml.find("<D:beta/>").unwrap();
        let alpha_second = xml.find("<D:alpha>second</D:alpha>").unwrap();
        assert!(alpha_first < beta && beta < alpha_second, "{xml}");
    }

    #[test]
    fn writes_list_then_raw_siblings_in_order() {
        let mut map = ValueMap::new();
        map.insert::<Alpha>(Value::List(Box::new(nonempty::nonempty![
            Value::Text("first".into()),
            Value::Text("second".into())
        ])));
        map.insert_raw(
            name(DAV_NAMESPACE, DAV_PREFIX, Alpha::LOCAL_NAME),
            Value::Text("third".into()),
        );

        let xml = render(Value::Map(map));
        assert!(xml.contains("<D:alpha>first</D:alpha>"), "{xml}");
        assert!(xml.contains("<D:alpha>second</D:alpha>"), "{xml}");
        assert!(xml.contains("<D:alpha>third</D:alpha>"), "{xml}");
        let first = xml.find("<D:alpha>first</D:alpha>").unwrap();
        let second = xml.find("<D:alpha>second</D:alpha>").unwrap();
        let third = xml.find("<D:alpha>third</D:alpha>").unwrap();
        assert!(first < second && second < third, "{xml}");
    }

    #[test]
    fn renames_colliding_prefixes() {
        let mut map = ValueMap::new();
        map.insert_raw(name("urn:one", "x", "a"), Value::Empty);
        map.insert_raw(name("urn:two", "x", "b"), Value::Empty);
        let xml = render(Value::Map(map));
        assert!(xml.contains("xmlns:x=\"urn:one\""), "{xml}");
        assert!(xml.contains("xmlns:ns0=\"urn:two\""), "{xml}");
        assert!(xml.contains("<ns0:b/>"), "{xml}");
    }

    #[test]
    fn prefix_less_namespace_gets_generated_prefix() {
        let mut map = ValueMap::new();
        map.insert_raw(
            ElementName {
                namespace: Some("urn:n".into()),
                prefix: None,
                local_name: "a".into(),
            },
            Value::Empty,
        );
        let xml = render(Value::Map(map));
        assert!(xml.contains("xmlns:ns0=\"urn:n\""), "{xml}");
    }

    #[test]
    fn escapes_text() {
        let mut map = ValueMap::new();
        map.insert_raw(name("DAV:", "D", "t"), Value::Text("a & <b>".into()));
        let xml = render(Value::Map(map));
        assert!(xml.contains("<D:t>a &amp; &lt;b&gt;</D:t>"), "{xml}");
    }

    #[test]
    fn empty_root_is_self_closing() {
        let xml = render(Value::Empty);
        assert!(xml.ends_with("<D:root xmlns:D=\"DAV:\"/>"), "{xml}");
    }

    #[test]
    fn text_root_is_written() {
        let xml = render(Value::Text("hi".into()));
        assert!(
            xml.ends_with("<D:root xmlns:D=\"DAV:\">hi</D:root>"),
            "{xml}"
        );
    }

    #[test]
    fn list_root_is_an_error() {
        let mut out = Vec::new();
        let list = Value::List(Box::new(nonempty::nonempty![Value::Empty]));
        let error = write_xml::<Root>(&mut out, list).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidValueType {
                element: "root",
                ..
            }
        ));
    }

    #[test]
    fn mixed_error_is_validated_as_an_error_container() {
        let mut map = ValueMap::new();
        map.insert_raw(
            name(DAV_NAMESPACE, DAV_PREFIX, DavError::LOCAL_NAME),
            Value::Mixed(vec![ContentItem::Element {
                name: name(DAV_NAMESPACE, DAV_PREFIX, "propfind-finite-depth"),
                value: Value::Empty,
            }]),
        );

        let mut out = Vec::new();
        let error = write_xml::<Root>(&mut out, Value::Map(map)).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidValueType {
                element: "error",
                expected: "child elements",
            }
        ));
    }

    #[test]
    fn preserves_opaque_dav_error_content_inside_owner() {
        let mut map = ValueMap::new();
        map.insert_raw(
            name(DAV_NAMESPACE, DAV_PREFIX, "owner"),
            Value::Mixed(vec![
                ContentItem::Text("\n  ".into()),
                ContentItem::Element {
                    name: name(DAV_NAMESPACE, DAV_PREFIX, DavError::LOCAL_NAME),
                    value: Value::Mixed(vec![
                        ContentItem::Text("\n    ".into()),
                        ContentItem::Element {
                            name: name(DAV_NAMESPACE, DAV_PREFIX, "propfind-finite-depth"),
                            value: Value::Empty,
                        },
                        ContentItem::Text("\n  ".into()),
                    ]),
                },
                ContentItem::Text("\n".into()),
            ]),
        );

        let mut out = Vec::new();
        write_xml::<Root>(&mut out, Value::Map(map)).unwrap();
    }

    #[test]
    fn indents_owner_start_while_preserving_owner_content() {
        let info = LockInfo::exclusive_write().with_owner(Owner::text("\n  Jane\n"));
        let mut out = Vec::new();
        write_xml::<LockInfo>(&mut out, info.into()).unwrap();

        assert_eq!(
            String::from_utf8(out).unwrap(),
            concat!(
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
                "<D:lockinfo xmlns:D=\"DAV:\">\n",
                "  <D:lockscope>\n",
                "    <D:exclusive/>\n",
                "  </D:lockscope>\n",
                "  <D:locktype>\n",
                "    <D:write/>\n",
                "  </D:locktype>\n",
                "  <D:owner>\n",
                "  Jane\n",
                "</D:owner>\n",
                "</D:lockinfo>"
            )
        );
    }

    #[test]
    fn unnamespaced_names_are_written_bare() {
        let mut map = ValueMap::new();
        map.insert_raw(
            ElementName {
                namespace: None,
                prefix: None,
                local_name: "plain".into(),
            },
            Value::Empty,
        );
        let xml = render(Value::Map(map));
        assert!(xml.contains("<plain/>"), "{xml}");
    }
}
