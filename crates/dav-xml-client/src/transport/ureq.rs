// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`Transport`] for [`ureq::Agent`].

use super::{Transport, TransportError, TransportErrorKind};

/// An agent that returns error statuses as responses, follows no redirects
/// (`WebDAV` clients must see `3xx` themselves), and allows the
/// non-standard HTTP methods `WebDAV` verbs need (`MKCOL`, `PROPFIND`,
/// `PROPPATCH`, `COPY`, `MOVE`, `LOCK`, `UNLOCK`), which `ureq` otherwise
/// rejects outright regardless of HTTP version.
///
/// # Examples
///
/// ```
/// use dav_xml_client::transport::ureq::default_agent;
///
/// let agent = default_agent();
/// assert!(!agent.config().http_status_as_error());
/// assert_eq!(agent.config().max_redirects(), 0);
/// assert!(agent.config().allow_non_standard_methods());
/// ```
#[must_use]
pub fn default_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .allow_non_standard_methods(true)
        .build()
        .into()
}

impl Transport for ureq::Agent {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        let response = self.run(request).map_err(convert)?;
        let (parts, mut body) = response.into_parts();
        let bytes = body
            .read_to_vec()
            .map_err(|error| TransportError::new(TransportErrorKind::Io, error))?;
        Ok(http::Response::from_parts(parts, bytes))
    }
}

/// Categorize a [`ureq::Error`] into a [`TransportErrorKind`].
fn convert(error: ureq::Error) -> TransportError {
    let kind = match &error {
        ureq::Error::ConnectionFailed | ureq::Error::HostNotFound => TransportErrorKind::Connect,
        ureq::Error::Timeout(_) => TransportErrorKind::Timeout,
        ureq::Error::Io(_) => TransportErrorKind::Io,
        _ => TransportErrorKind::Other,
    };
    TransportError::new(kind, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_refused_is_a_connect_error() {
        let agent = default_agent();
        let request = http::Request::builder()
            .uri("http://127.0.0.1:9/")
            .body(Vec::new())
            .unwrap();
        let error = agent.send(request).unwrap_err();
        assert!(
            matches!(
                error.kind(),
                TransportErrorKind::Connect | TransportErrorKind::Io
            ),
            "{error}"
        );
    }

    #[test]
    fn error_statuses_are_returned_not_raised() {
        let config = default_agent().config().clone();
        assert!(!config.http_status_as_error());
    }
}
