// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! [`AsyncTransport`] for [`reqwest::Client`].

use super::{AsyncTransport, TransportError, TransportErrorKind};

/// A client that follows no redirects (`WebDAV` clients must see `3xx`
/// themselves).
///
/// This only differs from [`reqwest::Client::new`] by its redirect policy;
/// every other setting (TLS, connection pooling, timeouts, ...) keeps
/// `reqwest`'s defaults.
///
/// # Examples
///
/// ```
/// use dav_xml_client::transport::reqwest::default_client;
///
/// let _client = default_client();
/// ```
#[must_use]
pub fn default_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_default()
}

impl AsyncTransport for reqwest::Client {
    async fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        let request = reqwest::Request::try_from(request).map_err(convert)?;
        let response = self.execute(request).await.map_err(convert)?;
        let mut builder = http::Response::builder()
            .status(response.status())
            .version(response.version());
        if let Some(headers) = builder.headers_mut() {
            headers.extend(response.headers().clone());
        }
        let body = response.bytes().await.map_err(convert)?;
        builder
            .body(body.to_vec())
            .map_err(|error| TransportError::new(TransportErrorKind::Other, error))
    }
}

/// Categorize a [`reqwest::Error`] into a [`TransportErrorKind`].
fn convert(error: reqwest::Error) -> TransportError {
    let kind = if error.is_connect() {
        TransportErrorKind::Connect
    } else if error.is_timeout() {
        TransportErrorKind::Timeout
    } else if error.is_body() || error.is_decode() {
        TransportErrorKind::Io
    } else {
        TransportErrorKind::Other
    };
    TransportError::new(kind, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connection_refused_is_a_connect_error() {
        let client = default_client();
        let request = http::Request::builder()
            .uri("http://127.0.0.1:9/")
            .body(Vec::new())
            .unwrap();
        let error = client.send(request).await.unwrap_err();
        assert!(
            matches!(
                error.kind(),
                TransportErrorKind::Connect | TransportErrorKind::Io
            ),
            "{error}"
        );
    }
}
