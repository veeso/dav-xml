// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Full verb sequence against a real server (`just containers_up`).

#![cfg(feature = "containers")]

use dav_xml::elements::{Depth, LockInfo, Prop, PropFind, PropertyUpdate, Timeout};
use dav_xml_client::headers::{If, Overwrite};
use dav_xml_client::{Auth, DavClient};
use serial_test::serial;

fn base() -> String {
    std::env::var("DAV_XML_TEST_URL").unwrap_or_else(|_| "http://localhost:3080".into())
}

fn auth() -> Auth {
    Auth::basic("alice", "secret1234")
}

fn unique(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{base}/dav-xml-{nanos}-{name}", base = base())
}

macro_rules! sync_suite {
    ($module:ident, $feature:literal, $client:expr) => {
        #[cfg(feature = $feature)]
        mod $module {
            use super::*;

            #[test]
            #[serial]
            fn full_sequence() {
                let client: DavClient<_> = $client;
                let dir = format!("{}/", unique("dir"));
                let file = format!("{dir}file.txt");
                let copy = format!("{dir}copy.txt");
                let moved = format!("{dir}moved.txt");

                let caps = client.options(&base()).unwrap();
                assert!(caps.supports_locking(), "{caps:?}");

                client.mkcol(&dir).unwrap();
                assert!(client.exists(&dir).unwrap());
                client.put(&file, b"hello".to_vec(), "text/plain").unwrap();

                let listing = client.list(&dir).unwrap();
                assert!(
                    listing
                        .iter()
                        .any(|r| r.name() == "file.txt" && r.content_length == Some(5)),
                    "{listing:?}"
                );

                let ms = client
                    .propfind(&file, &PropFind::PropName, Depth::Zero)
                    .unwrap();
                assert_eq!(ms.response.len(), 1);

                // `bytemark/webdav` (Apache mod_dav) rejects `PROPPATCH` of
                // `displayname` with a `403` propstat, so this exercises a
                // custom dead property in a private namespace instead.
                let update = PropertyUpdate::new().set(
                    Prop::builder()
                        .raw(
                            dav_xml::ElementName {
                                namespace: Some("https://dav-xml.veeso.dev/".into()),
                                prefix: Some("X".into()),
                                local_name: "note".into(),
                            },
                            dav_xml::Value::Text("renamed".into()),
                        )
                        .build(),
                );
                let ms = client.proppatch(&file, &update).unwrap();
                assert_eq!(ms.failures().count(), 0, "{ms:?}");

                let lock = client
                    .lock(
                        &file,
                        &LockInfo::exclusive_write(),
                        Some(Timeout::Seconds(60)),
                        Depth::Zero,
                    )
                    .unwrap();
                assert!(
                    client
                        .put(&file, b"blocked".to_vec(), "text/plain")
                        .is_err(),
                    "locked resource must reject unconditional PUT"
                );
                let locked = client.clone().with_if(If::untagged(lock.token.0.clone()));
                locked.put(&file, b"hello!".to_vec(), "text/plain").unwrap();
                client
                    .refresh_lock(&file, &lock.token, Some(Timeout::Seconds(120)))
                    .unwrap();
                client.unlock(&file, &lock.token).unwrap();

                client
                    .copy(&file, &copy, Depth::Zero, Overwrite::False)
                    .unwrap();
                assert!(
                    client
                        .copy(&file, &copy, Depth::Zero, Overwrite::False)
                        .is_err(),
                    "overwrite F must fail"
                );
                client.mv(&copy, &moved, Overwrite::True).unwrap();
                assert!(!client.exists(&copy).unwrap());
                assert_eq!(client.get(&moved).unwrap().body(), b"hello!");
                assert_eq!(client.head(&moved).unwrap().status(), 200);
                assert!(client.stat(&moved).unwrap().is_some());

                client.delete(&dir).unwrap();
                assert!(!client.exists(&dir).unwrap());
            }
        }
    };
}

sync_suite!(ureq, "ureq", DavClient::ureq(auth()));
sync_suite!(isahc_sync, "isahc", DavClient::isahc(auth()).unwrap());

macro_rules! async_suite {
    ($module:ident, $feature:literal, $client:expr) => {
        #[cfg(feature = $feature)]
        mod $module {
            use dav_xml_client::AsyncDavClient;

            use super::*;

            #[tokio::test]
            #[serial]
            async fn full_sequence() {
                let client: AsyncDavClient<_> = $client;
                let dir = format!("{}/", unique("adir"));
                let file = format!("{dir}file.txt");
                client.mkcol(&dir).await.unwrap();
                client
                    .put(&file, b"hello".to_vec(), "text/plain")
                    .await
                    .unwrap();
                let listing = client.list(&dir).await.unwrap();
                assert!(listing.iter().any(|r| r.name() == "file.txt"));
                let lock = client
                    .lock(
                        &file,
                        &LockInfo::exclusive_write(),
                        Some(Timeout::Seconds(60)),
                        Depth::Zero,
                    )
                    .await
                    .unwrap();
                client.unlock(&file, &lock.token).await.unwrap();
                let moved = format!("{dir}moved.txt");
                client.mv(&file, &moved, Overwrite::True).await.unwrap();
                assert_eq!(client.get(&moved).await.unwrap().body(), b"hello");
                client.delete(&dir).await.unwrap();
                assert!(!client.exists(&dir).await.unwrap());
            }
        }
    };
}

async_suite!(reqwest, "reqwest", AsyncDavClient::reqwest(auth()).unwrap());
async_suite!(isahc_async, "isahc", AsyncDavClient::isahc(auth()).unwrap());
