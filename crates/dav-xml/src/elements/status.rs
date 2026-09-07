// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt::Display;
use std::str::FromStr;

use bytestring::ByteString;

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// The `status` XML element as defined in RFC 4918 section 14.28.
///
/// Holds a full HTTP status line such as `HTTP/1.1 200 OK`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// The status code.
    pub code: http::StatusCode,
    /// The HTTP version named in the status line.
    pub version: http::Version,
    /// The reason phrase, if any.
    pub reason: Option<ByteString>,
}

impl Status {
    /// Construct an HTTP/1.1 status with the canonical reason phrase.
    #[must_use]
    pub fn new(code: http::StatusCode) -> Self {
        Self {
            code,
            version: http::Version::HTTP_11,
            reason: code.canonical_reason().map(Into::into),
        }
    }

    /// Whether the status code is in the 2xx range.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.code.is_success()
    }
}

impl Element for Status {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "status";
}

impl From<http::StatusCode> for Status {
    fn from(code: http::StatusCode) -> Self {
        Self::new(code)
    }
}

impl FromStr for Status {
    type Err = InvalidStatus;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || InvalidStatus(s.to_owned());
        let mut parts = s.trim().splitn(3, ' ');
        let version = match parts.next().ok_or_else(invalid)? {
            "HTTP/0.9" => http::Version::HTTP_09,
            "HTTP/1.0" => http::Version::HTTP_10,
            "HTTP/1.1" => http::Version::HTTP_11,
            "HTTP/2" | "HTTP/2.0" => http::Version::HTTP_2,
            "HTTP/3" | "HTTP/3.0" => http::Version::HTTP_3,
            _ => return Err(invalid()),
        };
        let code = parts
            .next()
            .and_then(|code| code.parse().ok())
            .ok_or_else(invalid)?;
        let reason = parts
            .next()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
            .map(Into::into);
        Ok(Self {
            code,
            version,
            reason,
        })
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{version:?} {code}",
            version = self.version,
            code = self.code.as_u16()
        )?;
        if let Some(reason) = &self.reason {
            write!(f, " {reason}")?;
        }
        Ok(())
    }
}

impl TryFrom<&Value> for Status {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map_err(Error::invalid::<Self>)
    }
}

impl From<Status> for Value {
    fn from(status: Status) -> Value {
        status.to_string().into()
    }
}

/// An HTTP status line that cannot be parsed.
#[derive(Debug, thiserror::Error)]
#[error("invalid status: {0}")]
pub struct InvalidStatus(String);

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn parses_status_line() {
        let status: Status = "HTTP/1.1 404 Not Found".parse().unwrap();
        assert_eq!(status.code, http::StatusCode::NOT_FOUND);
        assert_eq!(status.version, http::Version::HTTP_11);
        assert_eq!(status.reason.as_deref(), Some("Not Found"));
    }

    #[test]
    fn parses_without_reason() {
        let status: Status = "HTTP/1.0 200".parse().unwrap();
        assert_eq!(status.version, http::Version::HTTP_10);
        assert_eq!(status.reason, None);
    }

    #[test]
    fn rejects_non_http_version() {
        "GARBAGE 200 OK".parse::<Status>().unwrap_err();
        "HTTP/9 200 OK".parse::<Status>().unwrap_err();
    }

    #[test]
    fn rejects_bad_code() {
        "HTTP/1.1 9999 X".parse::<Status>().unwrap_err();
        "HTTP/1.1".parse::<Status>().unwrap_err();
    }

    #[test]
    fn displays_canonical_line() {
        assert_eq!(
            Status::new(http::StatusCode::OK).to_string(),
            "HTTP/1.1 200 OK"
        );
    }

    #[test]
    fn round_trips_through_value() {
        let value: Value = Status::new(http::StatusCode::FORBIDDEN).into();
        assert_eq!(
            Status::try_from(&value).unwrap().code,
            http::StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn value_error_names_status() {
        let error = Status::try_from(&Value::Text("nope".into())).unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidElement {
                element: "status",
                ..
            }
        ));
    }
}
