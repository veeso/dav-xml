// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `WebDAV` (RFC 4918) XML elements, properties and (de)serialization.
//!
//! The crate is populated by the migration tasks that follow.

/// The `DAV:` namespace URI.
///
/// # Examples
///
/// ```
/// assert_eq!(dav_xml::DAV_NAMESPACE, "DAV:");
/// ```
pub const DAV_NAMESPACE: &str = "DAV:";
