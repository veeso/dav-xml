# dav-xml-client

Minimal sync and async `WebDAV` ([RFC 4918](https://www.rfc-editor.org/rfc/rfc4918))
client for Rust, built on top of the `dav-xml` crate. `DavClient` (sync) and
`AsyncDavClient` (async) share one method set, one error type, and one
request/response pipeline; only the HTTP transport underneath changes.
Bring your own HTTP stack by implementing `Transport` or `AsyncTransport`, or
enable one of the built-in backends behind a Cargo feature.

## Feature flags

| name         | description                                                                            | default |
| ------------ | -------------------------------------------------------------------------------------- | ------- |
| `reqwest`    | `AsyncDavClient::reqwest`, an async backend for `tokio`.                               | ✔       |
| `ureq`       | `DavClient::ureq`, a blocking backend with no async runtime.                           |         |
| `isahc`      | `DavClient::isahc` and `AsyncDavClient::isahc`, libcurl for either sync or async code. |         |
| `native-tls` | Switches the `reqwest` and `ureq` backends to the platform TLS stack.                  |         |
| `rustls`     | No-op: `reqwest` and `ureq` already default to rustls (see below).                     |         |
| `mock`       | `transport::MockTransport`, for downstream tests.                                      |         |
| `containers` | Docker-backed integration tests (`tests/containers.rs`); pulls in `mock`.              |         |

`isahc` always builds with its own bundled `default-tls`, unaffected by
`native-tls` or `rustls`. For `reqwest` and `ureq`, rustls is already the
default (`reqwest`'s own `default-tls` feature resolves to rustls, and
`ureq`'s `TlsProvider` defaults to rustls too), so this crate's `rustls`
feature does nothing; enable `native-tls` to force the native TLS backend
for both instead. Enabling both `native-tls` and `rustls` together (as
`--all-features` does) resolves to `native-tls`.

## Choosing a backend

Use `DavClient::ureq` for synchronous code with no async runtime. Use
`AsyncDavClient::reqwest` on `tokio`. Use `DavClient::isahc` or
`AsyncDavClient::isahc` for libcurl, sync or async, on any executor.
Implement `Transport` (sync) or `AsyncTransport` (async) directly to plug in
another HTTP stack, or to layer proxying, retries, or tracing that the
built-in backends do not provide.

## Usage

### Sync, `ureq`

```rust,no_run
# #[cfg(feature = "ureq")]
# fn main() -> Result<(), dav_xml_client::Error> {
use dav_xml_client::{Auth, DavClient};

let client = DavClient::ureq(Auth::basic("alice", "secret"));
if !client.exists("https://dav.example.com/file.txt")? {
    client.put(
        "https://dav.example.com/file.txt",
        b"hello".to_vec(),
        "text/plain",
    )?;
}
# Ok(())
# }
#
# #[cfg(not(feature = "ureq"))]
# fn main() {}
```

### Async, `reqwest`

```rust,no_run
# #[cfg(feature = "reqwest")]
# #[tokio::main]
# async fn main() -> Result<(), dav_xml_client::Error> {
use dav_xml_client::{AsyncDavClient, Auth};

let client = AsyncDavClient::reqwest(Auth::basic("alice", "secret"))?;
for resource in client.list("https://dav.example.com/dir/").await? {
    println!("{path}", path = resource.path());
}
# Ok(())
# }
#
# #[cfg(not(feature = "reqwest"))]
# fn main() {}
```

### Custom transport

No backend feature is required: implement `Transport` (or `AsyncTransport`)
for any HTTP client.

```rust
use dav_xml_client::transport::{Transport, TransportError};
use dav_xml_client::{Auth, DavClient};

struct AlwaysNoContent;

impl Transport for AlwaysNoContent {
    fn send(
        &self,
        _request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        Ok(http::Response::builder()
            .status(204)
            .body(Vec::new())
            .unwrap())
    }
}

let client = DavClient::new(AlwaysNoContent, Auth::None);
client.delete("https://dav.example.com/file.txt")?;
# Ok::<(), dav_xml_client::Error>(())
```

## RFC 4918 coverage

| Section | Method    | Client call                                    | Result              |
| ------- | --------- | ---------------------------------------------- | ------------------- |
| 9.1     | PROPFIND  | `propfind(url, &PropFind, Depth)`              | `Multistatus`       |
| 9.2     | PROPPATCH | `proppatch(url, &PropertyUpdate)`              | `Multistatus`       |
| 9.3     | MKCOL     | `mkcol(url)`                                   | `()`                |
| 9.4     | GET       | `get(url)`                                     | `Response<Vec<u8>>` |
| 9.4     | HEAD      | `head(url)`                                    | `Response<()>`      |
| 9.6     | DELETE    | `delete(url)`                                  | `()`                |
| 9.7     | PUT       | `put(url, body, content_type)`                 | `()`                |
| 9.8     | COPY      | `copy(src, dst, Depth, Overwrite)`             | `()`                |
| 9.9     | MOVE      | `mv(src, dst, Overwrite)`                      | `()`                |
| 9.10    | LOCK      | `lock(url, &LockInfo, Option<Timeout>, Depth)` | `Lock`              |
| 9.11    | UNLOCK    | `unlock(url, &LockTokenHeader)`                | `()`                |
| 10.1    | OPTIONS   | `options(url)`                                 | `Capabilities`      |

A lock is refreshed with `refresh_lock(url, &LockTokenHeader,
Option<Timeout>)`, which reissues `LOCK` without a body (section 9.10.2).
`list(url)` and `stat(url)` are convenience helpers built on `propfind`;
`exists(url)` is built on `head`. None of the three are RFC methods in their
own right.

`delete`, `copy`, and `mv` return `()` on success; a `207 Multi-Status`
response listing per-resource failures surfaces as `Error::Multistatus`. A
lock token is attached to any subsequent request by cloning the client
(transports are cheap to clone) and calling `.with_if(cond)` with an `If`.

### Not covered

- Digest authentication (`Auth` supports `Basic` and `Bearer` only).
- Streaming request or response bodies; every body is buffered in memory.
- RFC 3744 (`WebDAV` Access Control Protocol).
- RFC 4331 (Quota and Size Properties).

## Installation

```toml
[dependencies]
dav-xml-client = "0.1" # async client, reqwest backend (default)
```

```toml
[dependencies]
dav-xml-client = { version = "0.1", default-features = false, features = ["ureq"] } # blocking client
```

Most `DavClient`/`AsyncDavClient` methods take `dav-xml` types (`Depth`,
`PropFind`, `LockInfo`, `Timeout`, ...). `dav-xml-client` re-exports the
whole crate as `dav_xml_client::dav_xml`, so `use dav_xml_client::dav_xml::elements::Depth;`
works without adding `dav-xml` as a separate dependency; depend on `dav-xml`
directly instead if you prefer importing it at its own path.

## License

Licensed under either of MIT or Apache-2.0 at your option.
