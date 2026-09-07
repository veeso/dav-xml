// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![expect(clippy::multiple_crate_versions, reason = "transitive dependencies")]

//! Minimal sync and async `WebDAV` (RFC 4918) client.
//!
//! The crate is populated by plan 3. It re-exports [`dav_xml`] so the
//! placeholder has a public item.
//!
//! # Examples
//!
//! ```
//! assert_eq!(dav_xml_client::DAV_NAMESPACE, "DAV:");
//! ```

pub use dav_xml::DAV_NAMESPACE;

pub mod auth;
pub mod headers;
pub mod transport;

#[doc(inline)]
pub use auth::Auth;
