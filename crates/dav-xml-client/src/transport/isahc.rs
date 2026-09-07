// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`Transport`] and [`AsyncTransport`] for [`isahc::HttpClient`].

use isahc::config::{Configurable, RedirectPolicy};
use isahc::{AsyncReadResponseExt, ReadResponseExt};

use super::{AsyncTransport, Transport, TransportError, TransportErrorKind};

/// A client that follows no redirects (`WebDAV` clients must see `3xx`
/// themselves).
///
/// This only differs from [`isahc::HttpClient::new`] by its redirect
/// policy; every other setting keeps `isahc`'s defaults.
///
/// # Errors
///
/// Returns a [`TransportError`] when libcurl cannot be initialised.
///
/// # Examples
///
/// ```
/// use dav_xml_client::transport::isahc::default_client;
///
/// let _client = default_client().unwrap();
/// ```
pub fn default_client() -> Result<isahc::HttpClient, TransportError> {
    isahc::HttpClient::builder()
        .redirect_policy(RedirectPolicy::None)
        .build()
        .map_err(convert)
}

impl Transport for isahc::HttpClient {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        let mut response = isahc::HttpClient::send(self, request).map_err(convert)?;
        let body = response
            .bytes()
            .map_err(|error| TransportError::new(TransportErrorKind::Io, error))?;
        let (parts, _body) = response.into_parts();
        Ok(http::Response::from_parts(parts, body))
    }
}

impl AsyncTransport for isahc::HttpClient {
    async fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        let mut response = self.send_async(request).await.map_err(convert)?;
        let body = response
            .bytes()
            .await
            .map_err(|error| TransportError::new(TransportErrorKind::Io, error))?;
        let (parts, _body) = response.into_parts();
        Ok(http::Response::from_parts(parts, body))
    }
}

/// Categorize an [`isahc::Error`] into a [`TransportErrorKind`].
fn convert(error: isahc::Error) -> TransportError {
    use isahc::error::ErrorKind;
    let kind = match error.kind() {
        ErrorKind::ConnectionFailed | ErrorKind::NameResolution => TransportErrorKind::Connect,
        ErrorKind::Timeout => TransportErrorKind::Timeout,
        ErrorKind::Io => TransportErrorKind::Io,
        _ => TransportErrorKind::Other,
    };
    TransportError::new(kind, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> http::Request<Vec<u8>> {
        http::Request::builder()
            .uri("http://127.0.0.1:9/")
            .body(Vec::new())
            .unwrap()
    }

    #[test]
    fn sync_connection_refused() {
        let client = default_client().unwrap();
        let error = Transport::send(&client, request()).unwrap_err();
        assert!(
            matches!(
                error.kind(),
                TransportErrorKind::Connect | TransportErrorKind::Io
            ),
            "{error}"
        );
    }

    #[tokio::test]
    async fn async_connection_refused() {
        let client = default_client().unwrap();
        let error = AsyncTransport::send(&client, request()).await.unwrap_err();
        assert!(
            matches!(
                error.kind(),
                TransportErrorKind::Connect | TransportErrorKind::Io
            ),
            "{error}"
        );
    }
}
