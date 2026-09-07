// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Every XML example of RFC 4918 parses, re-serializes and parses again.
//!
//! ## Fixture provenance
//!
//! `9.9.5-multistatus.xml` keeps the plan-mandated filename, but contains the
//! XML body from RFC 4918 section 9.9.6: section 9.9.5 has only a header-only
//! response. Likewise, `9.10.10-error.xml` contains the RFC 4918 section 16
//! condition example, because RFC 4918 has no section 9.10.10.

use dav_xml::elements::{
    DavError, LockInfo, Multistatus, Prop, PropFind, PropertyUpdate, Response,
};
use dav_xml::{FromXml, IntoXml};
use pretty_assertions::assert_eq;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{root}/tests/fixtures/rfc4918/{name}",
        root = env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

fn round_trip<T>(name: &str) -> T
where
    T: FromXml + IntoXml + Clone + PartialEq + std::fmt::Debug,
{
    let parsed = T::from_xml(fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let again = T::from_xml(parsed.clone().into_xml().unwrap()).unwrap();
    assert_eq!(again, parsed, "{name} round trip");
    parsed
}

#[test]
fn propfind_examples() {
    assert!(matches!(
        round_trip::<PropFind>("9.1.3-propfind-prop.xml"),
        PropFind::Prop(_)
    ));
    assert_eq!(
        round_trip::<PropFind>("9.1.4-propfind-propname.xml"),
        PropFind::PropName
    );
    assert!(matches!(
        round_trip::<PropFind>("9.1.5-propfind-allprop.xml"),
        PropFind::AllProp { include: None }
    ));
    let PropFind::AllProp {
        include: Some(include),
    } = round_trip::<PropFind>("9.1.6-propfind-include.xml")
    else {
        panic!()
    };
    assert_eq!(include.0.len(), 2);
}

#[test]
fn propfind_responses() {
    let ms = round_trip::<Multistatus>("9.1.3-multistatus.xml");
    assert_eq!(ms.response.len(), 1);
    let Response::Propstat { propstat, .. } = &ms.response[0] else {
        panic!()
    };
    assert_eq!(propstat.len(), 2);
    assert!(propstat[0].status.is_success());
    assert_eq!(propstat[1].status.code.as_u16(), 403);

    let ms = round_trip::<Multistatus>("9.1.4-multistatus.xml");
    assert_eq!(ms.response.len(), 2);

    let ms = round_trip::<Multistatus>("9.1.5-multistatus.xml");
    assert_eq!(ms.response.len(), 2);
    let Response::Propstat { propstat, .. } = &ms.response[0] else {
        panic!()
    };
    assert!(
        propstat[0]
            .prop
            .resourcetype()
            .unwrap()
            .unwrap()
            .unwrap()
            .is_collection()
    );
    assert_eq!(
        propstat[0]
            .prop
            .supportedlock()
            .unwrap()
            .unwrap()
            .unwrap()
            .0
            .len(),
        2
    );
}

#[test]
fn proppatch_examples() {
    let update = round_trip::<PropertyUpdate>("9.2.2-propertyupdate.xml");
    assert_eq!(update.0.len(), 2);
    let ms = round_trip::<Multistatus>("9.2.2-multistatus.xml");
    assert_eq!(ms.failures().count(), 2);
}

#[test]
fn delete_copy_move_examples() {
    assert_eq!(
        round_trip::<Multistatus>("9.6.2-multistatus.xml")
            .failures()
            .count(),
        1
    );
    assert_eq!(
        round_trip::<Multistatus>("9.8.8-multistatus.xml")
            .failures()
            .count(),
        1
    );
    let ms = round_trip::<Multistatus>("9.9.5-multistatus.xml");
    assert_eq!(ms.failures().count(), 1);
    assert!(
        ms.response[0]
            .error()
            .unwrap()
            .contains("lock-token-submitted")
    );
}

#[test]
fn lock_examples() {
    let info = round_trip::<LockInfo>("9.10.7-lockinfo.xml");
    assert!(info.owner.is_some());
    let prop = round_trip::<Prop>("9.10.7-prop-lockdiscovery.xml");
    let ld = prop.lockdiscovery().unwrap().unwrap().unwrap();
    assert_eq!(ld.0.len(), 1);
    assert_eq!(ld.0[0].timeout.unwrap().to_string(), "Second-604800");
    let prop = round_trip::<Prop>("9.10.8-prop-lockdiscovery.xml");
    assert_eq!(prop.lockdiscovery().unwrap().unwrap().unwrap().0.len(), 1);
    let ms = round_trip::<Multistatus>("9.10.9-multistatus.xml");
    assert_eq!(ms.failures().count(), 2);
    let error = round_trip::<DavError>("9.10.10-error.xml");
    assert!(error.contains("lock-token-submitted"));
}

#[test]
fn property_examples() {
    let prop = round_trip::<Prop>("15.8-lockdiscovery.xml");
    assert_eq!(prop.lockdiscovery().unwrap().unwrap().unwrap().0.len(), 1);
    let prop = round_trip::<Prop>("15.10-supportedlock.xml");
    assert_eq!(prop.supportedlock().unwrap().unwrap().unwrap().0.len(), 2);
}
