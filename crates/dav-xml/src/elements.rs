// SPDX-FileCopyrightText: d-k-bo <d-k-bo@mailbox.org>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XML element definitions from [RFC 4918 section 14](https://www.rfc-editor.org/rfc/rfc4918#section-14).

mod activelock;
mod allprop;
mod collection;
mod depth;
mod error;
mod exclusive;
mod href;
mod include;
mod location;
mod lockentry;
mod lockinfo;
mod lockroot;
mod lockscope;
mod locktoken;
mod locktype;
mod multistatus;
mod owner;
mod prop;
mod propertyupdate;
mod propfind;
mod propname;
mod propstat;
mod remove;
mod response;
mod responsedescription;
mod set;
mod shared;
mod status;
mod timeout;
mod write;

pub use self::activelock::ActiveLock;
pub use self::allprop::AllProp;
pub use self::collection::Collection;
pub use self::depth::{Depth, InvalidDepth};
pub use self::error::{Condition, DavError};
pub use self::exclusive::Exclusive;
pub use self::href::Href;
pub use self::include::Include;
pub use self::location::Location;
pub use self::lockentry::LockEntry;
pub use self::lockinfo::LockInfo;
pub use self::lockroot::LockRoot;
pub use self::lockscope::LockScope;
pub use self::locktoken::LockToken;
pub use self::locktype::LockType;
pub use self::multistatus::Multistatus;
pub use self::owner::Owner;
pub use self::prop::{Prop, PropBuilder};
pub use self::propertyupdate::{PropertyUpdate, PropertyUpdateItem};
pub use self::propfind::PropFind;
pub use self::propname::PropName;
pub use self::propstat::Propstat;
pub use self::remove::Remove;
pub use self::response::Response;
pub use self::responsedescription::ResponseDescription;
pub use self::set::Set;
pub use self::shared::Shared;
pub use self::status::{InvalidStatus, Status};
pub use self::timeout::{InvalidTimeout, Timeout};
pub use self::write::Write;
