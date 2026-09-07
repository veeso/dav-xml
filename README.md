# dav-xml

[![CI](https://github.com/veeso/dav-xml/actions/workflows/ci.yml/badge.svg)](https://github.com/veeso/dav-xml/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Crates.io dav-xml](https://img.shields.io/crates/v/dav-xml.svg?logo=rust&label=dav-xml)](https://crates.io/crates/dav-xml)
[![Crates.io dav-xml-client](https://img.shields.io/crates/v/dav-xml-client.svg?logo=rust&label=dav-xml-client)](https://crates.io/crates/dav-xml-client)
[![Docs dav-xml](https://img.shields.io/docsrs/dav-xml?label=docs%20dav-xml)](https://docs.rs/dav-xml)
[![Docs dav-xml-client](https://img.shields.io/docsrs/dav-xml-client?label=docs%20dav-xml-client)](https://docs.rs/dav-xml-client)

Rust WebDAV toolkit ([RFC 4918](https://www.rfc-editor.org/rfc/rfc4918)):
XML (de)serialization and a minimal HTTP client.

| Crate                                               | Description                                 |
| --------------------------------------------------- | ------------------------------------------- |
| [`dav-xml`](crates/dav-xml/README.md)               | XML elements, properties, (de)serialization |
| [`dav-xml-client`](crates/dav-xml-client/README.md) | Sync and async WebDAV client                |

## Quick start

### `dav-xml`

```rust,no_run
use dav_xml::FromXml;
use dav_xml::elements::Multistatus;

let xml = std::fs::read("multistatus.xml")?;
let multistatus = Multistatus::from_xml(xml)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

See the [`dav-xml` README](crates/dav-xml/README.md) for a fuller example
and for extending the property set.

### `dav-xml-client`

```rust,no_run
use dav_xml_client::{Auth, DavClient};

let client = DavClient::ureq(Auth::basic("alice", "secret"));
let exists = client.exists("https://dav.example.com/file.txt")?;
# let _ = exists;
# Ok::<(), dav_xml_client::Error>(())
```

See the [`dav-xml-client` README](crates/dav-xml-client/README.md) for the
feature matrix, backend choices, and async examples.

## RFC 4918 coverage

| Section | Items                                       | Status  |
| ------- | ------------------------------------------- | ------- |
| 9       | 11 of 12 methods (`POST`, 9.5, not covered) | Partial |
| 10      | All headers, including `OPTIONS` (10.1)     | Full    |
| 14      | All 30 XML elements                         | Full    |
| 15      | All 10 live properties                      | Full    |
| 16      | All 7 pre- and postcondition codes          | Full    |

### Not covered

- Digest authentication
- Streaming request or response bodies
- RFC 3744
- RFC 4331
- RFC 3253
- `CalDAV`
- `CardDAV`

## License

Licensed under either of MIT or Apache-2.0 at your option. The `dav-xml`
crate derives from the [`webdav-xml`](https://codeberg.org/d-k-bo/webdav-xml)
crate by d-k-bo, also MIT OR Apache-2.0.
