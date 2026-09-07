// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use nonempty::NonEmpty;

use crate::elements::{Href, Propstat, ResponseDescription, Status};
use crate::utils::NonEmptyExt;
use crate::value::ValueMap;
use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `response` XML element as defined in [RFC 4918](http://webdav.org/specs/rfc4918.html#ELEMENT_response).
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    /// A response containing one or more property statuses.
    Propstat {
        /// The resource URI.
        href: Href,
        /// Status values for groups of properties.
        propstat: NonEmpty<Propstat>,
        // error: Option<Error>,
        /// An optional human-readable description.
        responsedescription: Option<ResponseDescription>,
        // location: Option<Location>,
    },
    /// A response containing one status for one or more resource URIs.
    Status {
        /// The resource URIs.
        href: NonEmpty<Href>,
        /// The resource status.
        status: Status,
        // error: Option<Error>,
        /// An optional human-readable description.
        responsedescription: Option<ResponseDescription>,
        // location: Option<Location>,
    },
}

impl Element for Response {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "response";
}

impl TryFrom<&Value> for Response {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let map = value.as_map_of::<Self>()?;

        match NonEmpty::try_collect(map.iter_all::<Propstat>())? {
            Some(propstat) => Ok(Self::Propstat {
                href: map.get_required::<Self, Href>()?,
                propstat,
                responsedescription: map.get().transpose()?,
            }),
            None => Ok(Self::Status {
                href: NonEmpty::try_collect(map.iter_all::<Href>())?.ok_or(
                    Error::MissingElement {
                        parent: Self::LOCAL_NAME,
                        element: Href::LOCAL_NAME,
                    },
                )?,
                status: map.get_required::<Self, Status>()?,
                responsedescription: map.get().transpose()?,
            }),
        }
    }
}

impl From<Response> for Value {
    fn from(response: Response) -> Value {
        let mut map = ValueMap::new();

        match response {
            Response::Propstat {
                href,
                propstat,
                responsedescription,
            } => {
                map.insert::<Href>(href.into());
                map.insert::<Propstat>(Value::List(Box::new(propstat.map(Value::from))));
                if let Some(responsedescription) = responsedescription {
                    map.insert::<ResponseDescription>(responsedescription.into());
                }
            }
            Response::Status {
                href,
                status,
                responsedescription,
            } => {
                map.insert::<Href>(Value::List(Box::new(href.map(Value::from))));
                map.insert::<Status>(status.into());
                if let Some(responsedescription) = responsedescription {
                    map.insert::<ResponseDescription>(responsedescription.into());
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
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_propstat_variant_with_multiple_propstats() {
        let xml = r#"<D:response xmlns:D="DAV:">
  <D:href>/a</D:href>
  <D:propstat><D:prop><D:displayname>a</D:displayname></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  <D:propstat><D:prop><D:getetag/></D:prop><D:status>HTTP/1.1 404 Not Found</D:status></D:propstat>
  <D:propstat><D:prop><D:foo/></D:prop><D:status>HTTP/1.1 403 Forbidden</D:status></D:propstat>
  <D:responsedescription>x</D:responsedescription>
</D:response>"#;
        let response = Response::from_xml(xml.as_bytes().to_vec()).unwrap();
        let Response::Propstat {
            href,
            propstat,
            responsedescription,
            ..
        } = response
        else {
            panic!("expected propstat response")
        };
        assert_eq!(href.path(), "/a");
        assert_eq!(propstat.len(), 3);
        assert_eq!(responsedescription.unwrap().0, "x");
    }

    #[test]
    fn parses_status_variant_with_multiple_hrefs() {
        let xml = r#"<D:response xmlns:D="DAV:">
  <D:href>/a</D:href><D:href>/b</D:href><D:href>/c</D:href>
  <D:status>HTTP/1.1 424 Failed Dependency</D:status>
</D:response>"#;
        let response = Response::from_xml(xml.as_bytes().to_vec()).unwrap();
        let Response::Status { href, status, .. } = response else {
            panic!("expected status response")
        };
        assert_eq!(href.len(), 3);
        assert_eq!(status.code.as_u16(), 424);
    }

    #[test]
    fn missing_href_is_reported() {
        let xml = r#"<D:response xmlns:D="DAV:"><D:status>HTTP/1.1 200 OK</D:status></D:response>"#;
        let error = Response::from_xml(xml.as_bytes().to_vec()).unwrap_err();
        assert!(matches!(
            error,
            Error::MissingElement {
                parent: "response",
                element: "href"
            }
        ));
    }

    #[test]
    fn round_trips_both_variants() {
        for xml in [
            r#"<D:response xmlns:D="DAV:"><D:href>/a</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>"#,
            r#"<D:response xmlns:D="DAV:"><D:href>/a</D:href><D:href>/b</D:href><D:status>HTTP/1.1 200 OK</D:status></D:response>"#,
        ] {
            let response = Response::from_xml(xml.as_bytes().to_vec()).unwrap();
            let output = response.clone().into_xml().unwrap();
            assert_eq!(Response::from_xml(output).unwrap(), response);
        }
    }
}
