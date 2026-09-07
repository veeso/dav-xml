// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`Transport`] for [`ureq::Agent`].

use super::{Transport, TransportError, TransportErrorKind};

/// An agent configured with the three settings `WebDAV` needs and `ureq`
/// does not default to:
///
/// - `http_status_as_error(false)`, so a `4xx`/`5xx` response comes back as
///   an [`http::Response`] this client can inspect ([`crate::Error::Status`])
///   instead of surfacing as a transport error.
/// - `max_redirects(0)`, so a `3xx` response reaches this client instead of
///   being resolved by `ureq` (`WebDAV` clients must see `3xx` themselves).
/// - `allow_non_standard_methods(true)`, so the non-standard HTTP methods
///   `WebDAV` verbs need (`MKCOL`, `PROPFIND`, `PROPPATCH`, `COPY`, `MOVE`,
///   `LOCK`, `UNLOCK`) are not rejected outright before the request is sent;
///   without it, `ureq` refuses these regardless of HTTP version.
///
/// A caller building their own [`ureq::Agent`] for [`crate::DavClient::ureq_with`]
/// must set all three explicitly, or the affected verbs and status codes
/// silently misbehave.
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
    let builder = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .allow_non_standard_methods(true);
    #[cfg(feature = "native-tls")]
    let builder = builder.tls_config(
        ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .build(),
    );
    builder.build().into()
}

impl Transport for ureq::Agent {
    /// Reads the response body without `ureq`'s default 10 MiB
    /// [`ureq::Body::read_to_vec`] limit, so a large body fails the same way
    /// the `isahc` and `reqwest` adapters do (buffered fully, bounded only by
    /// available memory) rather than surfacing as an opaque
    /// [`TransportErrorKind::Io`] indistinguishable from a real I/O failure.
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        let response = self.run(request).map_err(convert)?;
        let (parts, mut body) = response.into_parts();
        let bytes = body
            .with_config()
            .limit(u64::MAX)
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

    #[test]
    fn bodies_larger_than_ureqs_default_10mib_limit_are_read_in_full() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpListener;

        // Exceeds `ureq::Body::read_to_vec`'s hardcoded default limit
        // (`MAX_BODY_SIZE = 10 * 1024 * 1024`); a `Transport` that used it
        // directly would fail this with an opaque `TransportErrorKind::Io`.
        let body_len = 11 * 1024 * 1024;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _addr) = listener.accept().unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            stream.write_all(&vec![b'a'; body_len]).unwrap();
            stream.flush().unwrap();
        });

        let agent = default_agent();
        let request = http::Request::builder()
            .uri(format!("http://{addr}/"))
            .body(Vec::new())
            .unwrap();
        let response = agent.send(request).unwrap();
        assert_eq!(response.body().len(), body_len);

        server.join().unwrap();
    }
}
