// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{FromXml, IntoXml};

    #[test]
    fn parses_second_and_infinite() {
        assert_eq!(
            "Second-604800".parse::<Timeout>().unwrap(),
            Timeout::Seconds(604_800)
        );
        assert_eq!("Infinite".parse::<Timeout>().unwrap(), Timeout::Infinite);
        assert_eq!("second-5".parse::<Timeout>().unwrap(), Timeout::Seconds(5));
    }

    #[test]
    fn rejects_garbage() {
        "Second-".parse::<Timeout>().unwrap_err();
        "Minute-5".parse::<Timeout>().unwrap_err();
        "Second-x".parse::<Timeout>().unwrap_err();
    }

    #[test]
    fn enforces_rfc_seconds_limit() {
        let maximum = u64::from(u32::MAX);
        assert_eq!(
            format!("Second-{maximum}").parse::<Timeout>().unwrap(),
            Timeout::Seconds(maximum)
        );
        format!("Second-{}", maximum + 1)
            .parse::<Timeout>()
            .unwrap_err();
    }

    #[test]
    fn displays() {
        assert_eq!(Timeout::Seconds(10).to_string(), "Second-10");
        assert_eq!(Timeout::Infinite.to_string(), "Infinite");
    }

    #[test]
    fn from_duration_rounds_down() {
        assert_eq!(
            Timeout::from_duration(Duration::from_millis(1500)),
            Timeout::Seconds(1)
        );
        assert_eq!(
            Timeout::from_duration(Duration::from_secs(u64::from(u32::MAX) + 1)),
            Timeout::Seconds(u64::from(u32::MAX))
        );
    }

    #[test]
    fn displays_only_rfc_valid_seconds() {
        assert_eq!(
            Timeout::Seconds(u64::from(u32::MAX) + 1).to_string(),
            format!("Second-{}", u64::from(u32::MAX))
        );
    }

    #[test]
    fn round_trips_through_xml() {
        let xml = Timeout::Seconds(3).into_xml().unwrap();
        assert_eq!(Timeout::from_xml(xml).unwrap(), Timeout::Seconds(3));
    }
}

use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

use crate::{DAV_NAMESPACE, DAV_PREFIX, Element, Error, Value};

/// A `WebDAV` lock timeout ([RFC 4918 section 14.29](https://www.rfc-editor.org/rfc/rfc4918#section-14.29)).
///
/// # Examples
///
/// ```
/// use dav_xml::elements::Timeout;
/// use std::str::FromStr;
///
/// assert_eq!(Timeout::from_str("Second-60").unwrap(), Timeout::Seconds(60));
/// assert_eq!(Timeout::Infinite.to_string(), "Infinite");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Timeout {
    /// The lock never expires on its own.
    Infinite,
    /// The lock expires after this many seconds.
    Seconds(u64),
}

impl Timeout {
    const MAX_SECONDS: u64 = u32::MAX as u64;

    /// Creates a timeout rounded down to whole seconds.
    #[must_use]
    pub const fn from_duration(duration: Duration) -> Self {
        let seconds = duration.as_secs();
        Self::Seconds(if seconds > Self::MAX_SECONDS {
            Self::MAX_SECONDS
        } else {
            seconds
        })
    }
}

impl Element for Timeout {
    const NAMESPACE: &'static str = DAV_NAMESPACE;
    const PREFIX: &'static str = DAV_PREFIX;
    const LOCAL_NAME: &'static str = "timeout";
}

impl FromStr for Timeout {
    type Err = InvalidTimeout;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("Infinite") {
            return Ok(Self::Infinite);
        }
        let (prefix, seconds) = s
            .split_once('-')
            .ok_or_else(|| InvalidTimeout(s.to_owned()))?;
        if !prefix.eq_ignore_ascii_case("Second") {
            return Err(InvalidTimeout(s.to_owned()));
        }
        let seconds = seconds
            .parse::<u64>()
            .map_err(|error| InvalidTimeout(format!("{s}: {error}")))?;
        if seconds > Self::MAX_SECONDS {
            return Err(InvalidTimeout(s.to_owned()));
        }
        Ok(Self::Seconds(seconds))
    }
}

impl Display for Timeout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infinite => f.write_str("Infinite"),
            Self::Seconds(seconds) => {
                write!(f, "Second-{}", (*seconds).min(Self::MAX_SECONDS))
            }
        }
    }
}

impl TryFrom<&Value> for Timeout {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        value
            .as_str_of::<Self>()?
            .parse()
            .map_err(Error::invalid::<Self>)
    }
}

impl From<Timeout> for Value {
    fn from(timeout: Timeout) -> Value {
        timeout.to_string().into()
    }
}

/// A timeout value that cannot be parsed according to RFC 4918.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid timeout: {0}")]
pub struct InvalidTimeout(String);
