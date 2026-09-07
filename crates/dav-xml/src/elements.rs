// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XML element definitions from [RFC 4918 section 14](https://www.rfc-editor.org/rfc/rfc4918#section-14).

mod allprop;
mod collection;
mod exclusive;
mod href;
mod include;
mod multistatus;
mod prop;
mod propfind;
mod propname;
mod propstat;
mod response;
mod responsedescription;
mod shared;
mod status;
mod write;

pub use self::allprop::AllProp;
pub use self::collection::Collection;
pub use self::exclusive::Exclusive;
pub use self::href::Href;
pub use self::include::Include;
pub use self::multistatus::Multistatus;
pub use self::prop::Prop;
pub use self::propfind::PropFind;
pub use self::propname::PropName;
pub use self::propstat::Propstat;
pub use self::response::Response;
pub use self::responsedescription::ResponseDescription;
pub use self::shared::Shared;
pub use self::status::{InvalidStatus, Status};
pub use self::write::Write;
