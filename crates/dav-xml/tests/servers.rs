// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bodies captured from real `WebDAV` servers.

use dav_xml::FromXml;
use dav_xml::elements::{Multistatus, Prop, Response};
use pretty_assertions::assert_eq;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{root}/tests/fixtures/servers/{name}",
        root = env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn first_prop(ms: &Multistatus) -> &Prop {
    let Response::Propstat { propstat, .. } = &ms.response[0] else {
        panic!()
    };
    &propstat[0].prop
}

#[test]
fn apache_mod_dav_listing() {
    let ms = Multistatus::from_xml(fixture("apache-mod_dav-propfind.xml")).unwrap();
    assert_eq!(ms.response.len(), 3);
    let prop = first_prop(&ms);
    assert!(
        prop.resourcetype()
            .unwrap()
            .unwrap()
            .unwrap()
            .is_collection()
    );
    prop.creationdate().unwrap().unwrap().unwrap();
    prop.getlastmodified().unwrap().unwrap().unwrap();
    assert!(prop.supportedlock().unwrap().unwrap().unwrap().0.len() >= 2);
    assert!(matches!(prop.lockdiscovery(), Some(None)));
    let Response::Propstat { propstat, .. } = &ms.response[2] else {
        panic!()
    };
    assert!(
        propstat[0]
            .prop
            .names()
            .any(|n| n.local_name == "executable")
    );
}

#[test]
fn nextcloud_listing() {
    let ms = Multistatus::from_xml(fixture("nextcloud-propfind.xml")).unwrap();
    assert_eq!(ms.response.len(), 2);
    let Response::Propstat { propstat, .. } = &ms.response[1] else {
        panic!()
    };
    let file = &propstat[0].prop;
    assert_eq!(file.getetag().unwrap().unwrap().unwrap().weak, false);
    assert!(file.getcontentlength().unwrap().unwrap().unwrap().0 > 0);
    assert!(
        file.names()
            .any(|n| n.namespace.as_deref() == Some("http://owncloud.org/ns"))
    );
    assert_eq!(ms.failures().count(), 2, "one 404 propstat per response");
}

#[test]
fn iis_listing_with_default_namespace() {
    let ms = Multistatus::from_xml(fixture("iis-propfind.xml")).unwrap();
    let prop = first_prop(&ms);
    prop.getcontentlength().unwrap().unwrap().unwrap();
    prop.getlastmodified().unwrap().unwrap().unwrap();
}

#[test]
fn apache_lock_response() {
    let prop = Prop::from_xml(fixture("apache-lock.xml")).unwrap();
    let ld = prop.lockdiscovery().unwrap().unwrap().unwrap();
    assert_eq!(
        ld.0[0].locktoken.as_ref().unwrap().0.scheme_str(),
        Some("opaquelocktoken")
    );
}
