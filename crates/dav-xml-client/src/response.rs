// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Interpretation of raw HTTP responses into typed results or errors.

use dav_xml::FromXml;
use dav_xml::elements::{DavError, Multistatus, Prop};
use dav_xml::properties::LockDiscovery;

use crate::capabilities::Capabilities;
use crate::client::Lock;
use crate::error::{Error, Result};
use crate::headers::{self, DavHeader, LockTokenHeader};

/// Whether the response's `Content-Type` header names an XML media type.
fn is_xml(response: &http::Response<Vec<u8>>) -> bool {
    response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|content_type| content_type.contains("xml"))
}

/// Pass a successful response through unchanged, or turn a non-success
/// status into a typed [`Error::Status`].
///
/// When the response carries an XML body, it is opportunistically parsed as
/// a `DAV:error` element; a body that fails to parse is kept raw instead of
/// failing this call.
///
/// # Errors
///
/// Returns [`Error::Status`] when `response` did not return a success
/// status.
pub(crate) fn ensure_success(response: http::Response<Vec<u8>>) -> Result<http::Response<Vec<u8>>> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let xml = is_xml(&response);
    let body = response.into_body();
    let error = xml.then(|| DavError::from_xml(body.clone()).ok()).flatten();
    Err(Error::Status {
        status,
        error,
        body,
    })
}

/// Parse a `207 Multi-Status` response body.
///
/// A response that answers `200 OK` with a multi-status body (some servers
/// do this) is reported as [`Error::Status`] with the raw body, so the
/// caller can still inspect it; only a response whose status is exactly
/// `207` is parsed.
///
/// # Errors
///
/// Returns [`Error::Status`] when `response` did not return a success
/// status or did not return `207 Multi-Status`, and [`Error::Xml`] when the
/// body cannot be parsed as a `multistatus` element.
pub(crate) fn multistatus(response: http::Response<Vec<u8>>) -> Result<Multistatus> {
    let response = ensure_success(response)?;
    if response.status() != http::StatusCode::MULTI_STATUS {
        return Err(Error::Status {
            status: response.status(),
            error: None,
            body: response.into_body(),
        });
    }
    Ok(Multistatus::from_xml(response.into_body())?)
}

/// Interpret a response to an operation that may report per-resource
/// failures through a `207 Multi-Status` body, such as `DELETE`, `COPY` or
/// `MOVE`.
///
/// Any other success status is `Ok(())`. A `207` response is `Ok(())` when
/// every reported status is successful, and `Err(Error::Multistatus(_))`
/// otherwise.
///
/// # Errors
///
/// Returns [`Error::Status`] when `response` did not return a success
/// status, [`Error::Xml`] when a `207` body cannot be parsed, and
/// [`Error::Multistatus`] when a `207` body reports at least one failure.
pub(crate) fn ok_or_multistatus(response: http::Response<Vec<u8>>) -> Result<()> {
    let response = ensure_success(response)?;
    if response.status() != http::StatusCode::MULTI_STATUS {
        return Ok(());
    }
    let multistatus = Multistatus::from_xml(response.into_body())?;
    if multistatus.failures().next().is_some() {
        Err(Error::Multistatus(multistatus))
    } else {
        Ok(())
    }
}

/// Read the `lockdiscovery` property and the `Lock-Token` response header
/// from a `LOCK` response.
///
/// # Errors
///
/// Returns [`Error::Status`] when `response` did not return a success
/// status, and [`Error::Xml`] when the body cannot be parsed as a `prop`
/// element.
pub(crate) fn lock_discovery(
    response: http::Response<Vec<u8>>,
) -> Result<(LockDiscovery, Option<LockTokenHeader>)> {
    let response = ensure_success(response)?;
    let token = response
        .headers()
        .get(&headers::LOCK_TOKEN)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<LockTokenHeader>().ok());
    let prop = Prop::from_xml(response.into_body())?;
    let discovery = match prop.lockdiscovery() {
        Some(Some(discovery)) => discovery?,
        _ => LockDiscovery::default(),
    };
    Ok((discovery, token))
}

/// Read the `DAV` and `Allow` response headers into [`Capabilities`].
///
/// Every occurrence of the `DAV` header is joined with `,` before parsing.
/// `Allow` tokens that are not valid HTTP methods are skipped rather than
/// failing the whole call.
///
/// # Errors
///
/// Returns [`Error::Status`] when `response` did not return a success
/// status.
pub(crate) fn capabilities(response: http::Response<Vec<u8>>) -> Result<Capabilities> {
    let response = ensure_success(response)?;
    let dav = response
        .headers()
        .get_all(&headers::DAV)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect::<Vec<_>>()
        .join(",")
        .parse::<DavHeader>()
        .unwrap_or_default();
    let allow = response
        .headers()
        .get_all(http::header::ALLOW)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|token| http::Method::from_bytes(token.trim().as_bytes()).ok())
        .collect();
    Ok(Capabilities { dav, allow })
}

/// Build a [`Lock`] from a `LOCK` or `LOCK` refresh response's
/// [`lock_discovery`] result.
///
/// The token is read, in order of preference, from `header` (the
/// response's `Lock-Token` header), then from the first `locktoken` found
/// in `discovery`, then from `fallback` (the token the caller already knew,
/// for a refresh whose response carries neither).
///
/// # Errors
///
/// Returns [`Error::MissingLockToken`] when none of `header`, `discovery`
/// or `fallback` carries a token.
pub(crate) fn lock_result(
    discovery: LockDiscovery,
    header: Option<LockTokenHeader>,
    fallback: Option<LockTokenHeader>,
) -> Result<Lock> {
    let token = header
        .or_else(|| {
            discovery
                .0
                .iter()
                .find_map(|lock| lock.locktoken.as_ref())
                .map(|token| LockTokenHeader(token.0.to_string()))
        })
        .or(fallback)
        .ok_or(Error::MissingLockToken)?;
    Ok(Lock { token, discovery })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn response(status: u16, content_type: &str, body: &str) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(status)
            .header("content-type", content_type)
            .body(body.as_bytes().to_vec())
            .unwrap()
    }

    const MS_FAIL: &str = r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/a</D:href><D:status>HTTP/1.1 423 Locked</D:status></D:response></D:multistatus>"#;
    const MS_OK: &str = r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/a</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;

    #[test]
    fn success_passes_through() {
        assert_eq!(ensure_success(response(204, "", "")).unwrap().status(), 204);
    }

    #[test]
    fn failure_with_dav_error_body_is_typed() {
        let body = r#"<D:error xmlns:D="DAV:"><D:lock-token-submitted><D:href>/a</D:href></D:lock-token-submitted></D:error>"#;
        let error =
            ensure_success(response(423, "application/xml; charset=utf-8", body)).unwrap_err();
        assert_eq!(error.status().unwrap(), 423);
        assert!(error.dav_error().unwrap().contains("lock-token-submitted"));
    }

    #[test]
    fn failure_with_html_body_keeps_raw_body() {
        let error = ensure_success(response(500, "text/html", "<html/>")).unwrap_err();
        let Error::Status {
            error: None, body, ..
        } = error
        else {
            panic!()
        };
        assert_eq!(body, b"<html/>");
    }

    #[test]
    fn failure_with_unparseable_xml_keeps_raw_body() {
        let error = ensure_success(response(400, "text/xml", "<broken")).unwrap_err();
        assert!(matches!(error, Error::Status { error: None, .. }));
    }

    #[test]
    fn multistatus_requires_207() {
        multistatus(response(200, "text/xml", MS_OK)).unwrap_err();
        assert_eq!(
            multistatus(response(207, "text/xml", MS_OK))
                .unwrap()
                .response
                .len(),
            1
        );
    }

    #[test]
    fn ok_or_multistatus_maps_failures() {
        ok_or_multistatus(response(204, "", "")).unwrap();
        ok_or_multistatus(response(207, "text/xml", MS_OK)).unwrap();
        let error = ok_or_multistatus(response(207, "text/xml", MS_FAIL)).unwrap_err();
        assert!(matches!(error, Error::Multistatus(ms) if ms.failures().count() == 1));
    }

    #[test]
    fn lock_discovery_reads_prop_and_header() {
        let body = r#"<D:prop xmlns:D="DAV:"><D:lockdiscovery><D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>infinity</D:depth><D:timeout>Second-604800</D:timeout><D:locktoken><D:href>urn:uuid:1</D:href></D:locktoken><D:lockroot><D:href>/a</D:href></D:lockroot></D:activelock></D:lockdiscovery></D:prop>"#;
        let mut resp = response(200, "application/xml", body);
        resp.headers_mut()
            .insert("lock-token", "<urn:uuid:1>".parse().unwrap());
        let (discovery, token) = lock_discovery(resp).unwrap();
        assert_eq!(discovery.0.len(), 1);
        assert_eq!(token.unwrap().0, "urn:uuid:1");
    }

    #[test]
    fn lock_discovery_without_header_is_none() {
        let body = r#"<D:prop xmlns:D="DAV:"><D:lockdiscovery/></D:prop>"#;
        let (discovery, token) = lock_discovery(response(200, "application/xml", body)).unwrap();
        assert!(discovery.0.is_empty());
        assert!(token.is_none());
    }
}
