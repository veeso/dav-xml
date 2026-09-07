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
use crate::{DAV_NAMESPACE, Error, Result, Value};

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
    match value {
        Value::List(list) => list.iter().try_for_each(|item| validate_value(name, item)),
        Value::Text(_) | Value::Empty => validate_error(name, value),
        Value::Map(map) => {
            validate_error(name, value)?;
            map.iter()
                .try_for_each(|(child, value)| validate_value(child, value))
        }
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
                for (child, value) in map.iter_ordered() {
                    self.write_value(child, value)?;
                }
                self.inner
                    .write_event(Event::End(BytesEnd::new(raw_name)))?;
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
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::DAV_PREFIX;
    use crate::elements::Condition;
    use crate::value::ValueMap;

    struct Root;

    impl Element for Root {
        const NAMESPACE: &'static str = DAV_NAMESPACE;
        const PREFIX: &'static str = DAV_PREFIX;
        const LOCAL_NAME: &'static str = "root";
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
