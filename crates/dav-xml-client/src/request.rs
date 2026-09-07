// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Builders for every RFC 4918 request.

use dav_xml::IntoXml;
use dav_xml::elements::{Depth, LockInfo, PropFind, PropertyUpdate, Timeout};

use crate::Auth;
use crate::error::{Error, Result};
use crate::headers::{self, If, IfList, LockTokenHeader, Overwrite};

/// The media type for every RFC 4918 XML request body.
pub(crate) const APPLICATION_XML: &str = "application/xml; charset=utf-8";

/// Builds an [`http::Method`] from a `&'static str`, falling back to `GET`
/// for the handful of names this module never calls it with.
fn method(name: &'static str) -> http::Method {
    http::Method::from_bytes(name.as_bytes()).unwrap_or(http::Method::GET)
}

/// The credentials, `If` precondition, and any caller-supplied extra headers
/// shared by every request built for one call.
pub(crate) struct RequestContext<'a> {
    /// Credentials attached as the `Authorization` header.
    pub auth: &'a Auth,
    /// A caller-supplied `If` precondition, applied to every request unless
    /// a specific builder overrides it (`refresh_lock` merges into it
    /// instead).
    pub if_header: Option<&'a If>,
    /// Additional headers applied after `Authorization` and `If`, replacing
    /// either one under the same name rather than duplicating it. Headers a
    /// specific verb builder adds after calling [`Self::base`] (`Depth`,
    /// `Destination`, `Timeout`, and so on) are set later still and are not
    /// affected by `extra`.
    pub extra: &'a http::HeaderMap,
}

impl RequestContext<'_> {
    /// Starts a request for `method` and `url`, applying auth, the `If`
    /// header, and the extra headers. An extra header replaces the
    /// `Authorization` or `If` header set here under the same name, rather
    /// than duplicating it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI, and
    /// [`Error::InvalidHeader`] when the `If` header renders to an invalid
    /// header value.
    fn base(&self, method: http::Method, url: &str) -> Result<http::request::Builder> {
        let uri: http::Uri = url.parse()?;
        let mut builder = http::Request::builder().method(method).uri(uri);
        if let Some(value) = self.auth.header_value() {
            builder = builder.header(http::header::AUTHORIZATION, value);
        }
        if let Some(cond) = self.if_header.filter(|c| !c.is_empty()) {
            builder = builder.header(headers::IF, http::HeaderValue::from_str(&cond.to_string())?);
        }
        // `HeaderMap::header` appends rather than replaces, so drop any
        // value this function already set for a name `extra` also carries
        // before adding `extra`'s own values; otherwise the caller's
        // override would be a second, unread value (`HeaderMap::get`
        // returns only the first) rather than a real override.
        if let Some(built) = builder.headers_mut() {
            for name in self.extra.keys() {
                built.remove(name);
            }
        }
        for (name, value) in self.extra {
            builder = builder.header(name, value);
        }
        Ok(builder)
    }

    /// Finishes `builder` with `body` serialized as an XML document, setting
    /// `Content-Type` to [`APPLICATION_XML`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] when `body` fails to serialize, and
    /// [`Error::Http`] when the request cannot be built.
    fn xml_body<T: IntoXml>(
        builder: http::request::Builder,
        body: T,
    ) -> Result<http::Request<Vec<u8>>> {
        let bytes = body.into_xml()?;
        Ok(builder
            .header(http::header::CONTENT_TYPE, APPLICATION_XML)
            .body(bytes.to_vec())?)
    }

    /// Finishes `builder` with an empty body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] when the request cannot be built.
    fn empty(builder: http::request::Builder) -> Result<http::Request<Vec<u8>>> {
        Ok(builder.body(Vec::new())?)
    }

    /// `OPTIONS` ([RFC 4918 section 9.1](https://www.rfc-editor.org/rfc/rfc4918#section-9.1)): discover the server's capabilities, with no body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI.
    pub fn options(&self, url: &str) -> Result<http::Request<Vec<u8>>> {
        Self::empty(self.base(http::Method::OPTIONS, url)?)
    }

    /// `PROPFIND` ([RFC 4918 section 9.1](https://www.rfc-editor.org/rfc/rfc4918#section-9.1)): retrieve properties, with the `Depth` header and a `propfind` body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI, and
    /// [`Error::Xml`] when `propfind` fails to serialize.
    pub fn propfind(
        &self,
        url: &str,
        propfind: &PropFind,
        depth: Depth,
    ) -> Result<http::Request<Vec<u8>>> {
        let builder = self
            .base(method("PROPFIND"), url)?
            .header(headers::DEPTH, depth.to_string());
        Self::xml_body(builder, propfind.clone())
    }

    /// `PROPPATCH` ([RFC 4918 section 9.2](https://www.rfc-editor.org/rfc/rfc4918#section-9.2)): set or remove properties, with a `propertyupdate` body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI, and
    /// [`Error::Xml`] when `update` fails to serialize.
    pub fn proppatch(&self, url: &str, update: &PropertyUpdate) -> Result<http::Request<Vec<u8>>> {
        let builder = self.base(method("PROPPATCH"), url)?;
        Self::xml_body(builder, update.clone())
    }

    /// `MKCOL` ([RFC 4918 section 9.3](https://www.rfc-editor.org/rfc/rfc4918#section-9.3)): create a collection, with no body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI.
    pub fn mkcol(&self, url: &str) -> Result<http::Request<Vec<u8>>> {
        Self::empty(self.base(method("MKCOL"), url)?)
    }

    /// `GET` ([RFC 4918 section 9.4](https://www.rfc-editor.org/rfc/rfc4918#section-9.4)): retrieve a resource's content, with no body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI.
    pub fn get(&self, url: &str) -> Result<http::Request<Vec<u8>>> {
        Self::empty(self.base(http::Method::GET, url)?)
    }

    /// `HEAD` ([RFC 4918 section 9.4](https://www.rfc-editor.org/rfc/rfc4918#section-9.4)): retrieve a resource's headers, with no body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI.
    pub fn head(&self, url: &str) -> Result<http::Request<Vec<u8>>> {
        Self::empty(self.base(http::Method::HEAD, url)?)
    }

    /// `PUT` ([RFC 4918 section 9.7](https://www.rfc-editor.org/rfc/rfc4918#section-9.7)): replace a resource's content with `body`, sent verbatim under `content_type`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI.
    pub fn put(
        &self,
        url: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<http::Request<Vec<u8>>> {
        Ok(self
            .base(http::Method::PUT, url)?
            .header(http::header::CONTENT_TYPE, content_type)
            .body(body)?)
    }

    /// `DELETE` ([RFC 4918 section 9.6](https://www.rfc-editor.org/rfc/rfc4918#section-9.6)): remove a resource, with no body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI.
    pub fn delete(&self, url: &str) -> Result<http::Request<Vec<u8>>> {
        Self::empty(self.base(http::Method::DELETE, url)?)
    }

    /// `COPY` ([RFC 4918 section 9.8](https://www.rfc-editor.org/rfc/rfc4918#section-9.8)): duplicate a resource to `destination`, with no body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` or `destination` is not a
    /// valid URI, and [`Error::InvalidArgument`] when `depth` is
    /// [`Depth::One`], which `COPY` does not permit.
    pub fn copy(
        &self,
        url: &str,
        destination: &str,
        depth: Depth,
        overwrite: Overwrite,
    ) -> Result<http::Request<Vec<u8>>> {
        if depth == Depth::One {
            return Err(Error::InvalidArgument("COPY depth must be 0 or infinity"));
        }
        let destination: http::Uri = destination.parse()?;
        let builder = self
            .base(method("COPY"), url)?
            .header(headers::DESTINATION, destination.to_string())
            .header(headers::DEPTH, depth.to_string())
            .header(headers::OVERWRITE, overwrite.to_string());
        Self::empty(builder)
    }

    /// `MOVE` ([RFC 4918 section 9.9](https://www.rfc-editor.org/rfc/rfc4918#section-9.9)): relocate a resource to `destination`, with no body.
    ///
    /// `MOVE` always acts on the whole resource, so unlike `COPY` it takes
    /// no `Depth` header.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` or `destination` is not a
    /// valid URI.
    pub fn mv(
        &self,
        url: &str,
        destination: &str,
        overwrite: Overwrite,
    ) -> Result<http::Request<Vec<u8>>> {
        let destination: http::Uri = destination.parse()?;
        let builder = self
            .base(method("MOVE"), url)?
            .header(headers::DESTINATION, destination.to_string())
            .header(headers::OVERWRITE, overwrite.to_string());
        Self::empty(builder)
    }

    /// `LOCK` ([RFC 4918 section 9.10](https://www.rfc-editor.org/rfc/rfc4918#section-9.10)): create a new lock, with a `lockinfo` body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::InvalidArgument`] when `depth` is [`Depth::One`], which
    /// `LOCK` does not permit, and [`Error::Xml`] when `info` fails to
    /// serialize.
    pub fn lock(
        &self,
        url: &str,
        info: &LockInfo,
        timeout: Option<Timeout>,
        depth: Depth,
    ) -> Result<http::Request<Vec<u8>>> {
        if depth == Depth::One {
            return Err(Error::InvalidArgument("LOCK depth must be 0 or infinity"));
        }
        let mut builder = self
            .base(method("LOCK"), url)?
            .header(headers::DEPTH, depth.to_string());
        if let Some(timeout) = timeout {
            builder = builder.header(headers::TIMEOUT, timeout.to_string());
        }
        Self::xml_body(builder, info.clone())
    }

    /// `LOCK` with an `If` header carrying `token` and no body ([RFC 4918 section 9.10.2](https://www.rfc-editor.org/rfc/rfc4918#section-9.10.2)): refresh an existing lock's timeout.
    ///
    /// When [`Self::if_header`] already carries a precondition, `token` is
    /// merged into a clone of it rather than replacing it, so a caller's own
    /// preconditions survive a lock refresh.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI, and
    /// [`Error::InvalidHeader`] when the merged `If` header renders to an
    /// invalid header value.
    pub fn refresh_lock(
        &self,
        url: &str,
        token: &LockTokenHeader,
        timeout: Option<Timeout>,
    ) -> Result<http::Request<Vec<u8>>> {
        let token_list = IfList::untagged().token(token.0.clone());
        let cond = match self.if_header {
            Some(existing) => existing.clone().with_list(token_list),
            None => If::new().with_list(token_list),
        };
        let mut builder = self.base(method("LOCK"), url)?;
        // `base` already applied `self.if_header` (if any); drop it so the
        // merged header below replaces it instead of appending a duplicate
        // `If` header value.
        if let Some(built) = builder.headers_mut() {
            built.remove(headers::IF);
        }
        builder = builder.header(headers::IF, http::HeaderValue::from_str(&cond.to_string())?);
        if let Some(timeout) = timeout {
            builder = builder.header(headers::TIMEOUT, timeout.to_string());
        }
        Self::empty(builder)
    }

    /// `UNLOCK` ([RFC 4918 section 9.11](https://www.rfc-editor.org/rfc/rfc4918#section-9.11)): remove a lock identified by `token`, with no body.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI.
    pub fn unlock(&self, url: &str, token: &LockTokenHeader) -> Result<http::Request<Vec<u8>>> {
        Self::empty(
            self.base(method("UNLOCK"), url)?
                .header(headers::LOCK_TOKEN, token.to_string()),
        )
    }
}

#[cfg(test)]
mod tests {
    use dav_xml::FromXml;
    use pretty_assertions::assert_eq;

    use super::*;

    fn ctx<'a>(
        auth: &'a Auth,
        if_header: Option<&'a If>,
        extra: &'a http::HeaderMap,
    ) -> RequestContext<'a> {
        RequestContext {
            auth,
            if_header,
            extra,
        }
    }

    fn header<'r>(request: &'r http::Request<Vec<u8>>, name: &str) -> Option<&'r str> {
        request.headers().get(name).and_then(|v| v.to_str().ok())
    }

    #[test]
    fn base_applies_auth_if_and_extra_headers() {
        let auth = Auth::basic("a", "b");
        let cond = If::untagged("t");
        let mut extra = http::HeaderMap::new();
        extra.insert("x-custom", "1".parse().unwrap());
        let request = ctx(&auth, Some(&cond), &extra).get("http://h/p").unwrap();
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(request.uri(), "http://h/p");
        assert_eq!(header(&request, "authorization"), Some("Basic YTpi"));
        assert_eq!(header(&request, "if"), Some("(<t>)"));
        assert_eq!(header(&request, "x-custom"), Some("1"));
    }

    #[test]
    fn extra_headers_override_authorization_and_if() {
        let auth = Auth::basic("a", "b");
        let cond = If::untagged("t");
        let mut extra = http::HeaderMap::new();
        extra.insert("authorization", "Bearer overridden".parse().unwrap());
        extra.insert("if", "(<override>)".parse().unwrap());
        let request = ctx(&auth, Some(&cond), &extra).get("http://h/p").unwrap();
        // A single value per name: the extra header replaced the one `base`
        // set, rather than appending a second, unread value.
        assert_eq!(request.headers().get_all("authorization").iter().count(), 1);
        assert_eq!(request.headers().get_all("if").iter().count(), 1);
        assert_eq!(header(&request, "authorization"), Some("Bearer overridden"));
        assert_eq!(header(&request, "if"), Some("(<override>)"));
    }

    #[test]
    fn invalid_url_is_reported() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        assert!(matches!(
            ctx(&auth, None, &extra).get("http://h /p"),
            Err(Error::InvalidUrl(_))
        ));
    }

    #[test]
    fn propfind_sets_depth_and_body() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let request = ctx(&auth, None, &extra)
            .propfind(
                "http://h/",
                &PropFind::AllProp { include: None },
                Depth::One,
            )
            .unwrap();
        assert_eq!(request.method().as_str(), "PROPFIND");
        assert_eq!(header(&request, "depth"), Some("1"));
        assert_eq!(header(&request, "content-type"), Some(APPLICATION_XML));
        assert_eq!(
            PropFind::from_xml(request.body().clone()).unwrap(),
            PropFind::AllProp { include: None }
        );
    }

    #[test]
    fn proppatch_sends_propertyupdate() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let update = PropertyUpdate::new().remove(
            dav_xml::elements::Prop::builder()
                .name::<dav_xml::properties::DisplayName>()
                .build(),
        );
        let request = ctx(&auth, None, &extra)
            .proppatch("http://h/x", &update)
            .unwrap();
        assert_eq!(request.method().as_str(), "PROPPATCH");
        assert_eq!(
            PropertyUpdate::from_xml(request.body().clone()).unwrap(),
            update
        );
    }

    #[test]
    fn mkcol_delete_head_have_no_body() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let c = ctx(&auth, None, &extra);
        for (request, method) in [
            (c.mkcol("http://h/d/").unwrap(), "MKCOL"),
            (c.delete("http://h/d/").unwrap(), "DELETE"),
            (c.head("http://h/f").unwrap(), "HEAD"),
        ] {
            assert_eq!(request.method().as_str(), method);
            assert!(request.body().is_empty());
            assert_eq!(header(&request, "depth"), None);
        }
    }

    #[test]
    fn put_sets_content_type_and_body() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let request = ctx(&auth, None, &extra)
            .put("http://h/f", b"hi".to_vec(), "text/plain")
            .unwrap();
        assert_eq!(request.method(), http::Method::PUT);
        assert_eq!(header(&request, "content-type"), Some("text/plain"));
        assert_eq!(request.body(), b"hi");
    }

    #[test]
    fn copy_and_move_set_destination_depth_overwrite() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let c = ctx(&auth, None, &extra);
        let copy = c
            .copy(
                "http://h/a",
                "http://h/b",
                Depth::Infinity,
                Overwrite::False,
            )
            .unwrap();
        assert_eq!(copy.method().as_str(), "COPY");
        assert_eq!(header(&copy, "destination"), Some("http://h/b"));
        assert_eq!(header(&copy, "depth"), Some("infinity"));
        assert_eq!(header(&copy, "overwrite"), Some("F"));
        let mv = c.mv("http://h/a", "http://h/b", Overwrite::True).unwrap();
        assert_eq!(mv.method().as_str(), "MOVE");
        assert_eq!(header(&mv, "overwrite"), Some("T"));
        assert_eq!(header(&mv, "depth"), None);
    }

    #[test]
    fn copy_rejects_depth_one() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        assert!(matches!(
            ctx(&auth, None, &extra).copy("http://h/a", "http://h/b", Depth::One, Overwrite::True),
            Err(Error::InvalidArgument(_))
        ));
    }

    #[test]
    fn lock_sends_lockinfo_timeout_depth() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let request = ctx(&auth, None, &extra)
            .lock(
                "http://h/a",
                &LockInfo::exclusive_write(),
                Some(Timeout::Seconds(60)),
                Depth::Zero,
            )
            .unwrap();
        assert_eq!(request.method().as_str(), "LOCK");
        assert_eq!(header(&request, "timeout"), Some("Second-60"));
        assert_eq!(header(&request, "depth"), Some("0"));
        assert_eq!(
            LockInfo::from_xml(request.body().clone()).unwrap(),
            LockInfo::exclusive_write()
        );
    }

    #[test]
    fn lock_rejects_depth_one() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        assert!(matches!(
            ctx(&auth, None, &extra).lock(
                "http://h/a",
                &LockInfo::exclusive_write(),
                None,
                Depth::One
            ),
            Err(Error::InvalidArgument(_))
        ));
    }

    #[test]
    fn refresh_lock_uses_if_header_and_no_body() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let token = LockTokenHeader("urn:uuid:1".into());
        let request = ctx(&auth, None, &extra)
            .refresh_lock("http://h/a", &token, Some(Timeout::Infinite))
            .unwrap();
        assert_eq!(request.method().as_str(), "LOCK");
        assert_eq!(header(&request, "if"), Some("(<urn:uuid:1>)"));
        assert_eq!(header(&request, "timeout"), Some("Infinite"));
        assert!(request.body().is_empty());
    }

    #[test]
    fn refresh_lock_merges_existing_if() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let existing = If::untagged("urn:uuid:existing");
        let token = LockTokenHeader("urn:uuid:1".into());
        let request = ctx(&auth, Some(&existing), &extra)
            .refresh_lock("http://h/a", &token, None)
            .unwrap();
        assert_eq!(request.method().as_str(), "LOCK");
        assert_eq!(
            header(&request, "if"),
            Some("(<urn:uuid:existing>) (<urn:uuid:1>)")
        );
    }

    #[test]
    fn unlock_sets_lock_token_header() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let request = ctx(&auth, None, &extra)
            .unlock("http://h/a", &LockTokenHeader("urn:uuid:1".into()))
            .unwrap();
        assert_eq!(request.method().as_str(), "UNLOCK");
        assert_eq!(header(&request, "lock-token"), Some("<urn:uuid:1>"));
    }

    #[test]
    fn options_has_no_body() {
        let auth = Auth::None;
        let extra = http::HeaderMap::new();
        let request = ctx(&auth, None, &extra).options("http://h/").unwrap();
        assert_eq!(request.method(), http::Method::OPTIONS);
    }
}
