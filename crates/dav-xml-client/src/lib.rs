// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![expect(clippy::multiple_crate_versions, reason = "transitive dependencies")]
#![doc = include_str!("../README.md")]

pub mod async_client;
pub mod auth;
pub mod capabilities;
pub mod client;
pub mod error;
pub mod headers;
pub(crate) mod request;
pub mod resource;
pub(crate) mod response;
pub mod transport;

#[doc(inline)]
pub use async_client::AsyncDavClient;
#[doc(inline)]
pub use auth::Auth;
#[doc(inline)]
pub use capabilities::Capabilities;
#[doc(inline)]
pub use client::{DavClient, Lock};
pub use dav_xml;
#[doc(inline)]
pub use error::{Error, Result};
#[doc(inline)]
pub use resource::Resource;
