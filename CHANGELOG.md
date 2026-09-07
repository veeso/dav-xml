# Changelog

All notable changes to this project are documented in this file.

## 0.1.0

Released on 2026-09-07

### Breaking changes

- **dav-xml:** represent opaque href values explicitly

> Href is now the Uri or Raw enum instead of a tuple struct.

### Added

- **dav-xml:** migrate webdav xml implementation
- **dav-xml:** add exclusive, shared and write elements
- **dav-xml:** add lockscope, locktype, depth and timeout elements
- **dav-xml:** add owner, lock token, lock entry and lockinfo elements
- **dav-xml:** add lock discovery support
- **dav-xml:** add error element with rfc 4918 condition codes
- **dav-xml:** carry error and location in responses
- **dav-xml:** add propertyupdate with ordered set and remove
- **dav-xml-client:** add transport traits and mock transport
- **dav-xml-client:** add auth and typed rfc 4918 headers
- **dav-xml-client:** add error type and response interpretation

> Add the crate's Error enum and Result alias, plus response helpers
> that turn an http::Response into typed success, multi-status, and
> lock-discovery results.

- **dav-xml-client:** build every rfc 4918 request
- **dav-xml-client:** add resource listing and capabilities types
- **dav-xml-client:** add the blocking client

> Wire DavClient<T> to RequestContext and the response interpreters for
> every RFC 4918 verb, plus list/stat/exists convenience methods built on
> propfind and head. Drop the dead_code expectations on request.rs and
> response.rs now that the client exercises them.

- **dav-xml-client:** add the async client
- **dav-xml-client:** add ureq transport

> Implement Transport for ureq::Agent with a default_agent() helper that
> disables the http-status-as-error and redirect-following behavior, and
> add DavClient::ureq/ureq_with constructors gated behind the ureq
> feature.

- **dav-xml-client:** add reqwest transport
- **dav-xml-client:** add isahc transport

> Implement Transport and AsyncTransport for isahc::HttpClient, the only
> backend supporting both blocking and async sends, plus fallible
> DavClient::isahc/isahc_with and AsyncDavClient::isahc/isahc_with
> constructors gated behind the isahc feature.

### Documentation

- **dav-xml:** add unit element examples
- **dav-xml:** link timeout and depth parse errors to RFC
- **dav-xml:** complete RFC links for parse errors
- **dav-xml:** add explicit depth RFC section link
- **dav-xml:** document lock elements and validate urn fallback
- **dav-xml:** document rfc 4918 coverage
- **dav-xml:** document RFC fixture provenance

### Fixed

- **dav-xml:** address RFC element review findings
- Breaking: **dav-xml:** represent opaque href values explicitly
- **dav-xml:** reject malformed href references
- **dav-xml:** align active lock serialization
- **dav-xml:** reject empty lock-token-submitted conditions
- **dav-xml:** validate nested error values before writing
- **dav-xml:** validate repeated error siblings individually
- **dav-xml:** preserve list order after raw insert
- **test:** separate documentation test arguments
- **dav-xml:** reject empty property updates
- **dav-xml:** preserve ordered value map occurrences
- **dav-xml:** preserve owner mixed content
- **dav-xml:** preserve owner whitespace and validate mixed errors
- **dav-xml:** indent nested owner start tag
- **dav-xml:** serialize remove properties as empty
- **dav-xml:** normalize adjacent owner text
- **dav-xml:** expand grouped fallback children
- **dav-xml:** preserve repeated remove property names
- **dav-xml:** preserve mixed content and owner validation
- **dav-xml:** preserve nested mixed property content
- **dav-xml:** preserve opaque property content
- **dav-xml:** preserve custom property whitespace
- **dav-xml:** preserve extension and unknown error content
- **dav-xml:** preserve unknown condition children
- **dav-xml:** validate repeated error values
- **dav-xml-client:** fix isahc tls feature and doc test coverage

> - Restore isahc/default-tls to isahc feature for usable default TLS backend
> - Keep TLS engine selection out of native-tls/rustls for isahc (avoids conflicts)
> - Reorder features alphabetically (containers before default)
> - Add --all-features to doc test invocation so MockTransport example runs
> - Verified all four feature combinations build clean

- **dav-xml-client:** fix timeout header error value and add missing doc examples
- **dav-xml-client:** make extra headers actually override
- **dav-xml:** make activelock lockroot optional for real-world interop

> RFC 4918 lists lockroot as required in activelock, but real-world
> servers (Apache mod_dav among them) omit it from LOCK responses,
> which made every such response unparseable.

- **dav-xml-client:** allow non-standard http methods in ureq transport

> ureq rejects any HTTP method outside a small built-in allowlist
> regardless of HTTP version, which silently broke every WebDAV verb
> (MKCOL, PROPFIND, PROPPATCH, COPY, MOVE, LOCK, UNLOCK) sent through
> the ureq backend. ureq exposes allow_non_standard_methods for exactly
> this case.

- **dav-xml-client:** send empty isahc request bodies as truly empty

> isahc's From<Vec<u8>> conversion always produces a zero-length buffer
> body, never its dedicated empty-body variant, so every HEAD request
> this client sends was configured as a custom request with an upload
> instead of libcurl's NOBODY mode. A compliant HEAD response then
> under-delivers the Content-Length it advertises, which libcurl reports
> as a partial transfer. Map an empty body to isahc's own empty body
> type so HEAD (and other bodyless verbs) is recognised correctly.

### Style

- **dav-xml:** clarify serialization errors and name timeout placeholders
