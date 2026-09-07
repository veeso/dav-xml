// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Credentials attached to outgoing requests.

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn basic_encodes_rfc7617_example() {
        let auth = Auth::basic("Aladdin", "open sesame");
        assert_eq!(
            auth.header_value().unwrap(),
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
    }

    #[test]
    fn base64_test_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected);
        }
    }

    #[test]
    fn bearer_and_none() {
        assert_eq!(Auth::bearer("tok").header_value().unwrap(), "Bearer tok");
        assert!(Auth::None.header_value().is_none());
    }

    #[test]
    fn debug_hides_secrets() {
        let text = format!("{:?}", Auth::basic("u", "p"));
        assert!(!text.contains('p'), "{text}");
    }
}

/// Credentials attached to every request.
///
/// # Examples
///
/// ```
/// use dav_xml_client::Auth;
///
/// let auth = Auth::basic("alice", "secret");
/// assert!(auth.header_value().unwrap().to_str().unwrap().starts_with("Basic "));
/// ```
#[derive(Clone, PartialEq, Eq)]
pub enum Auth {
    /// Send no `Authorization` header.
    None,
    /// HTTP Basic (RFC 7617).
    Basic {
        /// User name.
        username: String,
        /// Password.
        password: String,
    },
    /// `Authorization: Bearer <token>`.
    Bearer(String),
}

impl Auth {
    /// Basic credentials built from a `username` and `password`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::Auth;
    ///
    /// let auth = Auth::basic("alice", "secret");
    /// assert_eq!(auth.header_value().unwrap(), "Basic YWxpY2U6c2VjcmV0");
    /// ```
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// A bearer token.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::Auth;
    ///
    /// let auth = Auth::bearer("tok");
    /// assert_eq!(auth.header_value().unwrap(), "Bearer tok");
    /// ```
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }

    /// The `Authorization` header value, if any.
    ///
    /// Returns `None` for [`Auth::None`] or if the value is not a valid
    /// header (control characters in the token).
    #[must_use]
    pub fn header_value(&self) -> Option<http::HeaderValue> {
        let text = match self {
            Self::None => return None,
            Self::Basic { username, password } => {
                format!(
                    "Basic {encoded}",
                    encoded = base64(format!("{username}:{password}").as_bytes())
                )
            }
            Self::Bearer(token) => format!("Bearer {token}"),
        };
        let mut value = http::HeaderValue::from_str(&text).ok()?;
        value.set_sensitive(true);
        Some(value)
    }
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Basic { username, .. } => f
                .debug_struct("Basic")
                .field("username", username)
                .field("secret", &"***")
                .finish(),
            Self::Bearer(_) => f.write_str("Bearer(***)"),
        }
    }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard (padded) base64 encoding, used to build the `Basic` credentials
/// text (RFC 4648 section 4).
pub(crate) fn base64(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let bytes = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);
        let index = |shift: u32| ALPHABET[((n >> shift) & 0x3f) as usize] as char;
        out.push(index(18));
        out.push(index(12));
        out.push(if chunk.len() > 1 { index(6) } else { '=' });
        out.push(if chunk.len() > 2 { index(0) } else { '=' });
    }
    out
}
