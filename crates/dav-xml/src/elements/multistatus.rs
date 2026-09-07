// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use nonempty::NonEmpty;

use crate::elements::response::Response;
use crate::elements::{Href, ResponseDescription, Status};
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `multistatus` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_multistatus).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Multistatus {
    /// Responses returned for the requested resources.
    pub response: Vec<Response>,
    /// An optional description of the overall response.
    pub responsedescription: Option<ResponseDescription>,
}

impl Multistatus {
    /// Iterate over every `href` whose status is not successful, including
    /// per-property failures inside `propstat`.
    pub fn failures(&self) -> impl Iterator<Item = (&Href, &Status)> {
        self.response.iter().flat_map(|response| match response {
            Response::Propstat { href, propstat, .. } => propstat
                .iter()
                .filter(|item| !item.status.is_success())
                .map(|item| (href, &item.status))
                .collect::<Vec<_>>(),
            Response::Status { href, status, .. } if !status.is_success() => {
                href.iter().map(|item| (item, status)).collect()
            }
            Response::Status { .. } => Vec::new(),
        })
    }
}

impl Element for Multistatus {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "multistatus";
}

impl TryFrom<&Value> for Multistatus {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        if value == &Value::Empty {
            return Ok(Self::default());
        }

        let map = value.as_map_of::<Self>()?;
        Ok(Self {
            response: map.get_all::<Response>()?,
            responsedescription: map.get().transpose()?,
        })
    }
}

impl From<Multistatus> for Value {
    fn from(
        Multistatus {
            response,
            responsedescription,
        }: Multistatus,
    ) -> Value {
        let mut map = ValueMap::new();

        map.insert::<Response>(
            match NonEmpty::collect(response.into_iter().map(Value::from)) {
                Some(responses) => Value::List(Box::new(responses)),
                None => Value::Empty,
            },
        );
        if let Some(responsedescription) = responsedescription {
            map.insert::<ResponseDescription>(responsedescription.into());
        }

        Value::Map(map)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    const THREE: &str = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response><D:href>/a</D:href><D:status>HTTP/1.1 200 OK</D:status></D:response>
  <D:response><D:href>/b</D:href><D:status>HTTP/1.1 404 Not Found</D:status></D:response>
  <D:response><D:href>/c</D:href><D:status>HTTP/1.1 200 OK</D:status></D:response>
  <D:responsedescription>done</D:responsedescription>
</D:multistatus>"#;

    #[test]
    fn parses_three_responses_flat() {
        let multistatus = Multistatus::from_xml(THREE.as_bytes().to_vec()).unwrap();
        assert_eq!(multistatus.response.len(), 3);
        assert_eq!(multistatus.responsedescription.unwrap().0, "done");
    }

    #[test]
    fn parses_empty_multistatus() {
        let multistatus =
            Multistatus::from_xml(br#"<D:multistatus xmlns:D="DAV:"/>"#.to_vec()).unwrap();
        assert!(multistatus.response.is_empty());
        let multistatus =
            Multistatus::from_xml(br#"<D:multistatus xmlns:D="DAV:"> </D:multistatus>"#.to_vec())
                .unwrap();
        assert!(multistatus.response.is_empty());
    }

    #[test]
    fn failures_lists_non_success() {
        let multistatus = Multistatus::from_xml(THREE.as_bytes().to_vec()).unwrap();
        let failing: Vec<_> = multistatus
            .failures()
            .map(|(href, status)| (href.0.path().to_owned(), status.code.as_u16()))
            .collect();
        assert_eq!(failing, [("/b".to_owned(), 404)]);
    }

    #[test]
    fn round_trips() {
        let multistatus = Multistatus::from_xml(THREE.as_bytes().to_vec()).unwrap();
        let xml = multistatus.clone().into_xml().unwrap();
        assert_eq!(Multistatus::from_xml(xml).unwrap(), multistatus);
    }
}
