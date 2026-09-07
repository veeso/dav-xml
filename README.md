# dav-xml

[![CI](https://github.com/veeso/dav-xml/actions/workflows/ci.yml/badge.svg)](https://github.com/veeso/dav-xml/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Rust WebDAV toolkit ([RFC 4918](https://www.rfc-editor.org/rfc/rfc4918)):
XML (de)serialization and a minimal HTTP client.

| Crate            | Description                                 |
| ---------------- | ------------------------------------------- |
| `dav-xml`        | XML elements, properties, (de)serialization |
| `dav-xml-client` | Sync and async WebDAV client                |

## RFC 4918 coverage

| Section | Items                              | Status |
| ------- | ---------------------------------- | ------ |
| 14      | All 30 XML elements                | Full   |
| 15      | All 10 live properties             | Full   |
| 16      | All 7 pre- and postcondition codes | Full   |

### Not covered

- RFC 3744
- RFC 4331
- RFC 3253
- `CalDAV`
- `CardDAV`

## License

Licensed under either of MIT or Apache-2.0 at your option. The `dav-xml`
crate derives from the [`webdav-xml`](https://codeberg.org/d-k-bo/webdav-xml)
crate by d-k-bo, also MIT OR Apache-2.0.
