// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bytestring::ByteString;

use crate::element::ElementName;
use crate::elements::Href;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value, ValueMap};

/// The `error` XML element
/// ([RFC 4918 section 14.5](https://www.rfc-editor.org/rfc/rfc4918#section-14.5)).
///
/// Named `DavError` to avoid clashing with [`crate::Error`].
///
/// # Examples
///
/// ```
/// use dav_xml::FromXml;
/// use dav_xml::elements::{Condition, DavError};
///
/// let xml = br#"<D:error xmlns:D="DAV:"><D:propfind-finite-depth/></D:error>"#;
/// let error = DavError::from_xml(xml.to_vec()).unwrap();
/// assert_eq!(error.conditions, vec![Condition::PropfindFiniteDepth]);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DavError {
    /// Pre- and postcondition codes, in document order.
    pub conditions: Vec<Condition>,
}

/// A precondition or postcondition code
/// ([RFC 4918 section 16](https://www.rfc-editor.org/rfc/rfc4918#section-16)).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::{Condition, Href};
///
/// let condition = Condition::LockTokenSubmitted(vec!["/locked".parse::<Href>().unwrap()]);
/// assert_eq!(condition.name(), "lock-token-submitted");
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Condition {
    /// `DAV:lock-token-matches-request-uri` (UNLOCK).
    LockTokenMatchesRequestUri,
    /// `DAV:lock-token-submitted`, with any locked resources supplied by the server.
    ///
    /// RFC 4918 section 16 requires one or more `href` children. For
    /// interoperability, this crate also accepts and serializes an empty value,
    /// because the published examples in sections 9.6.2, 9.8.8, and 9.9.6 use
    /// an empty `lock-token-submitted` element.
    LockTokenSubmitted(Vec<Href>),
    /// `DAV:no-conflicting-lock`, with the conflicting resources if known.
    NoConflictingLock(Vec<Href>),
    /// `DAV:no-external-entities`.
    NoExternalEntities,
    /// `DAV:preserved-live-properties`.
    PreservedLiveProperties,
    /// `DAV:propfind-finite-depth`.
    PropfindFiniteDepth,
    /// `DAV:cannot-modify-protected-property`.
    CannotModifyProtectedProperty,
    /// Any condition this crate does not model, kept verbatim.
    Other {
        /// Qualified name of the condition element.
        name: ElementName<ByteString>,
        /// Its content.
        value: Value,
    },
}

impl Condition {
    /// The local name of the condition element.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::LockTokenMatchesRequestUri => "lock-token-matches-request-uri",
            Self::LockTokenSubmitted(_) => "lock-token-submitted",
            Self::NoConflictingLock(_) => "no-conflicting-lock",
            Self::NoExternalEntities => "no-external-entities",
            Self::PreservedLiveProperties => "preserved-live-properties",
            Self::PropfindFiniteDepth => "propfind-finite-depth",
            Self::CannotModifyProtectedProperty => "cannot-modify-protected-property",
            Self::Other { name, .. } => &name.local_name,
        }
    }

    fn hrefs(value: &Value) -> Result<Vec<Href>, Error> {
        match value {
            Value::Empty => Ok(Vec::new()),
            other => other.as_map().and_then(ValueMap::get_all::<Href>),
        }
    }

    fn from_entry(name: &ElementName<ByteString>, value: &Value) -> Result<Self, Error> {
        if name.namespace.as_deref() != Some(DAV_NAMESPACE) {
            return Ok(Self::Other {
                name: name.clone(),
                value: value.clone(),
            });
        }

        Ok(match &*name.local_name {
            "lock-token-matches-request-uri" => Self::LockTokenMatchesRequestUri,
            "lock-token-submitted" => Self::LockTokenSubmitted(Self::hrefs(value)?),
            "no-conflicting-lock" => Self::NoConflictingLock(Self::hrefs(value)?),
            "no-external-entities" => Self::NoExternalEntities,
            "preserved-live-properties" => Self::PreservedLiveProperties,
            "propfind-finite-depth" => Self::PropfindFiniteDepth,
            "cannot-modify-protected-property" => Self::CannotModifyProtectedProperty,
            _ => Self::Other {
                name: name.clone(),
                value: value.clone(),
            },
        })
    }

    fn into_entry(self) -> (ElementName<ByteString>, Value) {
        fn dav(local: &str) -> ElementName<ByteString> {
            ElementName {
                namespace: Some(DAV_NAMESPACE.into()),
                prefix: Some(DAV_PREFIX.into()),
                local_name: local.to_owned().into(),
            }
        }

        match self {
            Self::LockTokenSubmitted(hrefs) => (
                dav("lock-token-submitted"),
                crate::value::list_value_under::<Href>(hrefs),
            ),
            Self::NoConflictingLock(hrefs) => (
                dav("no-conflicting-lock"),
                crate::value::list_value_under::<Href>(hrefs),
            ),
            Self::Other { name, value } => (name, value),
            unit => (dav(unit.name()), Value::Empty),
        }
    }
}

impl DavError {
    /// An error with one condition.
    #[must_use]
    pub fn single(condition: Condition) -> Self {
        Self {
            conditions: vec![condition],
        }
    }

    /// Whether a condition with local name `name` is present.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.conditions
            .iter()
            .any(|condition| condition.name() == name)
    }
}

impl Element for DavError {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "error";
}

impl TryFrom<&Value> for DavError {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = match value {
            Value::Empty => return Ok(Self::default()),
            other => other.as_map_of::<Self>()?,
        };
        let mut conditions = Vec::new();
        for (name, value) in map.iter() {
            match value {
                Value::List(list) => {
                    for item in list.iter() {
                        conditions.push(Condition::from_entry(name, item)?);
                    }
                }
                single => conditions.push(Condition::from_entry(name, single)?),
            }
        }
        Ok(Self { conditions })
    }
}

impl From<DavError> for Value {
    fn from(error: DavError) -> Value {
        if error.conditions.is_empty() {
            return Value::Empty;
        }

        let mut map = ValueMap::new();
        for condition in error.conditions {
            let (name, value) = condition.into_entry();
            map.insert_raw(name, value);
        }
        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    const RFC_9_10_10: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:error xmlns:D="DAV:">
  <D:lock-token-submitted>
    <D:href>/workspace/webdav/</D:href>
  </D:lock-token-submitted>
</D:error>"#;

    #[test]
    fn parses_lock_token_submitted_with_hrefs() {
        let error = DavError::from_xml(RFC_9_10_10.as_bytes().to_vec()).unwrap();
        assert_eq!(error.conditions.len(), 1);
        let Condition::LockTokenSubmitted(hrefs) = &error.conditions[0] else {
            panic!();
        };
        assert_eq!(hrefs[0].path(), "/workspace/webdav/");
        assert!(error.contains("lock-token-submitted"));
    }

    #[test]
    fn parses_every_named_condition() {
        for (name, expected) in [
            (
                "lock-token-matches-request-uri",
                Condition::LockTokenMatchesRequestUri,
            ),
            ("no-external-entities", Condition::NoExternalEntities),
            (
                "preserved-live-properties",
                Condition::PreservedLiveProperties,
            ),
            ("propfind-finite-depth", Condition::PropfindFiniteDepth),
            (
                "cannot-modify-protected-property",
                Condition::CannotModifyProtectedProperty,
            ),
        ] {
            let xml = format!(r#"<D:error xmlns:D="DAV:"><D:{name}/></D:error>"#);
            let error = DavError::from_xml(xml.into_bytes()).unwrap();
            assert_eq!(error.conditions, vec![expected]);
        }
    }

    #[test]
    fn no_conflicting_lock_may_have_no_hrefs() {
        let error = DavError::from_xml(
            br#"<D:error xmlns:D="DAV:"><D:no-conflicting-lock/></D:error>"#.to_vec(),
        )
        .unwrap();
        assert_eq!(
            error.conditions,
            vec![Condition::NoConflictingLock(Vec::new())]
        );
    }

    #[test]
    fn accepts_empty_lock_token_submitted_from_rfc_examples() {
        let error = DavError::from_xml(
            br#"<D:error xmlns:D="DAV:"><D:lock-token-submitted/></D:error>"#.to_vec(),
        )
        .unwrap();
        assert_eq!(
            error.conditions,
            vec![Condition::LockTokenSubmitted(Vec::new())]
        );
    }

    #[test]
    fn serializes_empty_lock_token_submitted_from_rfc_examples() {
        let error = DavError::single(Condition::LockTokenSubmitted(Vec::new()));
        assert_eq!(
            DavError::from_xml(error.clone().into_xml().unwrap()).unwrap(),
            error
        );
    }

    #[test]
    fn parses_multiple_lock_token_submitted_hrefs() {
        let error = DavError::from_xml(
            br#"<D:error xmlns:D="DAV:"><D:lock-token-submitted><D:href>/a</D:href><D:href>/b</D:href></D:lock-token-submitted></D:error>"#.to_vec(),
        )
        .unwrap();
        let Condition::LockTokenSubmitted(hrefs) = &error.conditions[0] else {
            panic!();
        };
        assert_eq!(
            hrefs.iter().map(Href::path).collect::<Vec<_>>(),
            vec!["/a", "/b"]
        );
    }

    #[test]
    fn round_trips_raw_lock_token_submitted_href() {
        let error = DavError::from_xml(
            br#"<D:error xmlns:D="DAV:"><D:lock-token-submitted><D:href>urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4</D:href></D:lock-token-submitted></D:error>"#.to_vec(),
        )
        .unwrap();
        assert!(matches!(
            error.conditions[0],
            Condition::LockTokenSubmitted(ref hrefs) if matches!(hrefs.as_slice(), [Href::Raw(value)] if value == "urn:uuid:e71d4fae-5dec-22d6-fea5-00a0c91e6be4")
        ));
        assert_eq!(
            DavError::from_xml(error.clone().into_xml().unwrap()).unwrap(),
            error
        );
    }

    #[test]
    fn unknown_conditions_are_kept() {
        let xml = br#"<D:error xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"><C:valid-calendar-data/></D:error>"#;
        let error = DavError::from_xml(xml.to_vec()).unwrap();
        assert_eq!(error.conditions[0].name(), "valid-calendar-data");
        assert!(
            matches!(&error.conditions[0], Condition::Other { name, .. } if name.namespace.as_deref() == Some("urn:ietf:params:xml:ns:caldav"))
        );
    }

    #[test]
    fn round_trips_unknown_condition_raw_value() {
        let xml = br#"<D:error xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"><C:valid-calendar-data>opaque-value</C:valid-calendar-data></D:error>"#;
        let error = DavError::from_xml(xml.to_vec()).unwrap();
        assert_eq!(
            DavError::from_xml(error.clone().into_xml().unwrap()).unwrap(),
            error
        );
    }

    #[test]
    fn empty_error_has_no_conditions() {
        let error = DavError::from_xml(br#"<D:error xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        assert!(error.conditions.is_empty());
    }

    #[test]
    fn round_trips() {
        let error = DavError::from_xml(RFC_9_10_10.as_bytes().to_vec()).unwrap();
        assert_eq!(
            DavError::from_xml(error.clone().into_xml().unwrap()).unwrap(),
            error
        );
        let other = DavError::single(Condition::PropfindFiniteDepth);
        assert_eq!(
            DavError::from_xml(other.clone().into_xml().unwrap()).unwrap(),
            other
        );
    }
}
