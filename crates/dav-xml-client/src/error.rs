// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The crate's error type and result alias.

use dav_xml::elements::{DavError, Multistatus};
use http::StatusCode;

use crate::transport::TransportError;

/// Result alias used across this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned while building requests or interpreting responses.
///
/// # Examples
///
/// ```
/// use dav_xml_client::Error;
///
/// let error = Error::InvalidArgument("depth must be 0, 1 or infinity");
/// assert!(error.status().is_none());
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A failure below the HTTP layer, such as a connection or timeout error.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The server answered with a non-success status.
    #[error("server returned {status}")]
    Status {
        /// The status returned by the server.
        status: StatusCode,
        /// The typed `DAV:error` body, when the response carried one.
        error: Option<DavError>,
        /// The raw response body.
        body: Vec<u8>,
    },
    /// A `207 Multi-Status` response reported one or more failures, for
    /// example on `DELETE`, `COPY` or `MOVE`.
    #[error("multi-status with {n} failures", n = _0.failures().count())]
    Multistatus(Multistatus),
    /// The response body could not be parsed as `WebDAV` XML.
    #[error(transparent)]
    Xml(#[from] dav_xml::Error),
    /// A URL supplied by the caller is not a valid URI.
    #[error(transparent)]
    InvalidUrl(#[from] http::uri::InvalidUri),
    /// A value supplied by the caller is not a valid header value.
    #[error(transparent)]
    InvalidHeader(#[from] http::header::InvalidHeaderValue),
    /// A caller-supplied argument is invalid, with a human-readable reason.
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
    /// A lock operation expected a lock token that the server did not
    /// return.
    #[error("missing lock token")]
    MissingLockToken,
    /// Building the HTTP request failed.
    #[error(transparent)]
    Http(#[from] http::Error),
}

impl Error {
    /// The HTTP status associated with this error, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::Error;
    ///
    /// assert!(Error::MissingLockToken.status().is_none());
    /// ```
    #[must_use]
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Status { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The typed `DAV:error` body associated with this error, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::Error;
    ///
    /// assert!(Error::MissingLockToken.dav_error().is_none());
    /// ```
    #[must_use]
    pub fn dav_error(&self) -> Option<&DavError> {
        match self {
            Self::Status { error, .. } => error.as_ref(),
            _ => None,
        }
    }
}
