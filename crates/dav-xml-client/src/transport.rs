// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! HTTP transport abstraction.
//!
//! A transport sends one [`http::Request`] and returns one
//! [`http::Response`] with the whole body in memory. Implement [`Transport`]
//! for a blocking client and [`AsyncTransport`] for an async one; the
//! feature-gated adapters in this module do so for `ureq`, `reqwest` and
//! `isahc`.

#[cfg(feature = "isahc")]
mod isahc;
#[cfg(any(test, feature = "mock"))]
mod mock;
#[cfg(feature = "reqwest")]
mod reqwest;
#[cfg(feature = "ureq")]
mod ureq;

#[cfg(any(test, feature = "mock"))]
pub use self::mock::MockTransport;

/// A blocking HTTP transport.
///
/// # Examples
///
/// ```
/// use dav_xml_client::transport::{Transport, TransportError};
///
/// struct Always204;
///
/// impl Transport for Always204 {
///     fn send(&self, _: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>, TransportError> {
///         Ok(http::Response::builder().status(204).body(Vec::new()).unwrap())
///     }
/// }
/// ```
pub trait Transport {
    /// Send `request` and wait for the full response.
    ///
    /// # Errors
    ///
    /// Any connection, timeout or I/O failure. HTTP error statuses are not
    /// errors at this level.
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError>;
}

/// An async HTTP transport.
pub trait AsyncTransport {
    /// Send `request` and resolve with the full response.
    ///
    /// # Errors
    ///
    /// See [`Transport::send`].
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> impl Future<Output = Result<http::Response<Vec<u8>>, TransportError>> + Send;
}

/// Category of a transport failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportErrorKind {
    /// The connection could not be established.
    Connect,
    /// The request timed out.
    Timeout,
    /// Reading or writing the stream failed.
    Io,
    /// Anything else.
    Other,
}

/// A failure below the HTTP layer.
#[derive(Debug, thiserror::Error)]
#[error("transport error ({kind:?}): {source}")]
pub struct TransportError {
    kind: TransportErrorKind,
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl TransportError {
    /// Wrap a backend error.
    pub fn new(
        kind: TransportErrorKind,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            kind,
            source: source.into(),
        }
    }

    /// The failure category.
    #[must_use]
    pub fn kind(&self) -> TransportErrorKind {
        self.kind
    }
}

impl<T: Transport + ?Sized> Transport for &T {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        (**self).send(request)
    }
}

impl<T: Transport + ?Sized> Transport for std::sync::Arc<T> {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        (**self).send(request)
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        (**self).send(request)
    }
}

impl<T: AsyncTransport + Sync + ?Sized> AsyncTransport for &T {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> impl Future<Output = Result<http::Response<Vec<u8>>, TransportError>> + Send {
        (**self).send(request)
    }
}

impl<T: AsyncTransport + Sync + ?Sized> AsyncTransport for std::sync::Arc<T> {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> impl Future<Output = Result<http::Response<Vec<u8>>, TransportError>> + Send {
        (**self).send(request)
    }
}

impl<T: AsyncTransport + Sync + ?Sized> AsyncTransport for Box<T> {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> impl Future<Output = Result<http::Response<Vec<u8>>, TransportError>> + Send {
        (**self).send(request)
    }
}
