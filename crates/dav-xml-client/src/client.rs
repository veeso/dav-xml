// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The blocking `WebDAV` client.

#[cfg(test)]
mod tests {
    use dav_xml::elements::Prop;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::transport::MockTransport;

    fn client() -> (DavClient<MockTransport>, MockTransport) {
        let mock = MockTransport::new();
        (DavClient::new(mock.clone(), Auth::basic("a", "b")), mock)
    }

    fn reply(mock: &MockTransport, status: u16, body: &str) {
        let mut builder = http::Response::builder().status(status);
        if !body.is_empty() {
            builder = builder.header("content-type", "application/xml");
        }
        mock.reply(builder.body(body.as_bytes().to_vec()).unwrap());
    }

    const LISTING: &str = r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/d/</D:href><D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response><D:response><D:href>/d/f</D:href><D:propstat><D:prop><D:getcontentlength>1</D:getcontentlength></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
    const LOCK_BODY: &str = r#"<D:prop xmlns:D="DAV:"><D:lockdiscovery><D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>0</D:depth><D:timeout>Second-60</D:timeout><D:locktoken><D:href>urn:uuid:1</D:href></D:locktoken><D:lockroot><D:href>/d/f</D:href></D:lockroot></D:activelock></D:lockdiscovery></D:prop>"#;

    #[test]
    fn list_sends_allprop_depth_one_and_flattens() {
        let (client, mock) = client();
        reply(&mock, 207, LISTING);
        let resources = client.list("http://h/d/").unwrap();
        let request = mock.last_request().unwrap();
        assert_eq!(request.method().as_str(), "PROPFIND");
        assert_eq!(request.headers()["depth"], "1");
        assert_eq!(resources.len(), 2);
        assert!(resources[0].is_collection);
    }

    #[test]
    fn stat_returns_none_on_404() {
        let (client, mock) = client();
        reply(&mock, 404, "");
        assert!(client.stat("http://h/x").unwrap().is_none());
        reply(&mock, 207, LISTING);
        assert!(client.stat("http://h/d/").unwrap().is_some());
        assert_eq!(mock.last_request().unwrap().headers()["depth"], "0");
    }

    #[test]
    fn exists_uses_head() {
        let (client, mock) = client();
        reply(&mock, 200, "");
        assert!(client.exists("http://h/f").unwrap());
        assert_eq!(mock.last_request().unwrap().method(), http::Method::HEAD);
        reply(&mock, 404, "");
        assert!(!client.exists("http://h/f").unwrap());
    }

    #[test]
    fn propfind_returns_multistatus() {
        let (client, mock) = client();
        reply(&mock, 207, LISTING);
        let ms = client
            .propfind("http://h/d/", &PropFind::PropName, Depth::Zero)
            .unwrap();
        assert_eq!(ms.response.len(), 2);
    }

    #[test]
    fn proppatch_returns_multistatus() {
        let (client, mock) = client();
        reply(&mock, 207, LISTING);
        let update = PropertyUpdate::new().remove(
            Prop::builder()
                .name::<dav_xml::properties::DisplayName>()
                .build(),
        );
        let ms = client.proppatch("http://h/d/", &update).unwrap();
        assert_eq!(ms.response.len(), 2);
        assert_eq!(mock.last_request().unwrap().method().as_str(), "PROPPATCH");
    }

    #[test]
    fn mkcol_put_delete_return_unit_on_2xx() {
        let (client, mock) = client();
        reply(&mock, 201, "");
        client.mkcol("http://h/d/").unwrap();
        reply(&mock, 204, "");
        client
            .put("http://h/d/f", b"x".to_vec(), "text/plain")
            .unwrap();
        reply(&mock, 204, "");
        client.delete("http://h/d/f").unwrap();
        assert_eq!(mock.requests().len(), 3);
        assert_eq!(client.transport().requests().len(), 3);
    }

    #[test]
    fn delete_207_with_failures_is_an_error() {
        let (client, mock) = client();
        reply(
            &mock,
            207,
            r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/d/f</D:href><D:status>HTTP/1.1 423 Locked</D:status></D:response></D:multistatus>"#,
        );
        assert!(matches!(
            client.delete("http://h/d/"),
            Err(Error::Multistatus(_))
        ));
    }

    #[test]
    fn copy_and_move_forward_headers() {
        let (client, mock) = client();
        reply(&mock, 201, "");
        client
            .copy(
                "http://h/a",
                "http://h/b",
                Depth::Infinity,
                Overwrite::False,
            )
            .unwrap();
        assert_eq!(mock.last_request().unwrap().headers()["overwrite"], "F");
        reply(&mock, 204, "");
        client
            .mv("http://h/a", "http://h/c", Overwrite::True)
            .unwrap();
        assert_eq!(
            mock.last_request().unwrap().headers()["destination"],
            "http://h/c"
        );
    }

    #[test]
    fn get_and_head_return_responses() {
        let (client, mock) = client();
        reply(&mock, 200, "");
        mock.reply(
            http::Response::builder()
                .status(200)
                .header("etag", "\"x\"")
                .body(b"body".to_vec())
                .unwrap(),
        );
        let head = client.head("http://h/f").unwrap();
        assert_eq!(head.status(), 200);
        let get = client.get("http://h/f").unwrap();
        assert_eq!(get.body(), b"body");
        assert_eq!(get.headers()["etag"], "\"x\"");
    }

    #[test]
    fn lock_refresh_unlock_flow() {
        let (client, mock) = client();
        mock.reply(
            http::Response::builder()
                .status(200)
                .header("content-type", "application/xml")
                .header("lock-token", "<urn:uuid:1>")
                .body(LOCK_BODY.as_bytes().to_vec())
                .unwrap(),
        );
        let lock = client
            .lock(
                "http://h/d/f",
                &LockInfo::exclusive_write(),
                Some(Timeout::Seconds(60)),
                Depth::Zero,
            )
            .unwrap();
        assert_eq!(lock.token.0, "urn:uuid:1");
        assert_eq!(lock.discovery.0[0].timeout, Some(Timeout::Seconds(60)));

        reply(&mock, 200, LOCK_BODY);
        let refreshed = client
            .refresh_lock("http://h/d/f", &lock.token, Some(Timeout::Seconds(120)))
            .unwrap();
        assert_eq!(
            refreshed.token.0, "urn:uuid:1",
            "token kept when header absent"
        );
        assert_eq!(
            mock.last_request().unwrap().headers()["if"],
            "(<urn:uuid:1>)"
        );

        reply(&mock, 204, "");
        client.unlock("http://h/d/f", &lock.token).unwrap();
        assert_eq!(
            mock.last_request().unwrap().headers()["lock-token"],
            "<urn:uuid:1>"
        );
    }

    #[test]
    fn lock_without_token_header_or_body_token_is_an_error() {
        let (client, mock) = client();
        reply(
            &mock,
            200,
            r#"<D:prop xmlns:D="DAV:"><D:lockdiscovery/></D:prop>"#,
        );
        assert!(matches!(
            client.lock(
                "http://h/f",
                &LockInfo::exclusive_write(),
                None,
                Depth::Zero
            ),
            Err(Error::MissingLockToken)
        ));
    }

    #[test]
    fn with_if_applies_to_every_request() {
        let (client, mock) = client();
        let client = client.with_if(If::untagged("urn:uuid:1"));
        reply(&mock, 204, "");
        client.put("http://h/f", Vec::new(), "text/plain").unwrap();
        assert_eq!(
            mock.last_request().unwrap().headers()["if"],
            "(<urn:uuid:1>)"
        );
    }

    #[test]
    fn with_header_sets_custom_header() {
        let (client, mock) = client();
        let client = client.with_header(
            http::HeaderName::from_static("x-custom"),
            http::HeaderValue::from_static("1"),
        );
        reply(&mock, 204, "");
        client.put("http://h/f", Vec::new(), "text/plain").unwrap();
        assert_eq!(mock.last_request().unwrap().headers()["x-custom"], "1");
    }

    #[test]
    fn status_errors_carry_dav_error() {
        let (client, mock) = client();
        reply(
            &mock,
            423,
            r#"<D:error xmlns:D="DAV:"><D:lock-token-submitted><D:href>/f</D:href></D:lock-token-submitted></D:error>"#,
        );
        let error = client
            .put("http://h/f", Vec::new(), "text/plain")
            .unwrap_err();
        assert_eq!(error.status().unwrap(), 423);
        assert!(error.dav_error().unwrap().contains("lock-token-submitted"));
    }

    #[test]
    fn transport_errors_are_wrapped() {
        let (client, _mock) = client();
        assert!(matches!(client.get("http://h/f"), Err(Error::Transport(_))));
    }

    #[test]
    fn options_returns_capabilities() {
        let (client, mock) = client();
        mock.reply(
            http::Response::builder()
                .status(200)
                .header("dav", "1,2")
                .header("allow", "GET")
                .body(Vec::new())
                .unwrap(),
        );
        let caps = client.options("http://h/").unwrap();
        assert!(caps.supports_locking());
    }
}

use dav_xml::elements::{Depth, LockInfo, Multistatus, PropFind, PropertyUpdate, Timeout};
use dav_xml::properties::LockDiscovery;
use http::{HeaderName, HeaderValue};

use crate::capabilities::Capabilities;
use crate::error::{Error, Result};
use crate::headers::{If, LockTokenHeader, Overwrite};
use crate::request::RequestContext;
use crate::resource::Resource;
use crate::transport::Transport;
use crate::{Auth, response};

/// A blocking `WebDAV` client.
///
/// Every method builds one request, sends it through `T`, and interprets
/// the response into a typed result or an [`Error`]. A precondition set
/// with [`DavClient::with_if`] and any header set with
/// [`DavClient::with_header`] apply to every request the client sends.
///
/// # Examples
///
/// ```
/// use dav_xml_client::transport::MockTransport;
/// use dav_xml_client::{Auth, DavClient};
///
/// let mock = MockTransport::new();
/// let body = br#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/notes/</D:href><D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
/// mock.reply(
///     http::Response::builder()
///         .status(207)
///         .header("content-type", "application/xml")
///         .body(body.to_vec())
///         .unwrap(),
/// );
/// let client = DavClient::new(mock, Auth::basic("alice", "secret"));
/// for resource in client.list("http://localhost:3080/").unwrap() {
///     println!("{path}", path = resource.path());
/// }
/// ```
#[derive(Clone, Debug)]
pub struct DavClient<T> {
    transport: T,
    auth: Auth,
    if_header: Option<If>,
    headers: http::HeaderMap,
}

/// The result of a successful `LOCK` request.
///
/// # Examples
///
/// ```
/// use dav_xml::properties::LockDiscovery;
/// use dav_xml_client::Lock;
/// use dav_xml_client::headers::LockTokenHeader;
///
/// let lock = Lock {
///     token: LockTokenHeader("urn:uuid:1".to_owned()),
///     discovery: LockDiscovery::default(),
/// };
/// assert_eq!(lock.token.0, "urn:uuid:1");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Lock {
    /// Token to send back in `If` and `Lock-Token` headers.
    pub token: LockTokenHeader,
    /// The `lockdiscovery` property returned by the server.
    pub discovery: LockDiscovery,
}

impl<T: Transport> DavClient<T> {
    /// Build a client sending every request through `transport`, using
    /// `auth` for the `Authorization` header.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let client = DavClient::new(MockTransport::new(), Auth::basic("alice", "secret"));
    /// assert_eq!(client.transport().requests().len(), 0);
    /// ```
    #[must_use]
    pub fn new(transport: T, auth: Auth) -> Self {
        Self {
            transport,
            auth,
            if_header: None,
            headers: http::HeaderMap::new(),
        }
    }

    /// Attach an `If` precondition, applied to every request this client
    /// sends afterward (a lock token acquired from [`DavClient::lock`], for
    /// example).
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::headers::If;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock.clone(), Auth::basic("a", "b"))
    ///     .with_if(If::untagged("urn:uuid:1"));
    /// client.put("http://localhost/f", Vec::new(), "text/plain").unwrap();
    /// assert_eq!(mock.last_request().unwrap().headers()["if"], "(<urn:uuid:1>)");
    /// ```
    #[must_use]
    pub fn with_if(mut self, cond: If) -> Self {
        self.if_header = Some(cond);
        self
    }

    /// Attach a header applied to every request this client sends
    /// afterward, replacing any value `base` would otherwise set under the
    /// same name.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock.clone(), Auth::basic("a", "b")).with_header(
    ///     http::HeaderName::from_static("x-custom"),
    ///     http::HeaderValue::from_static("1"),
    /// );
    /// client.put("http://localhost/f", Vec::new(), "text/plain").unwrap();
    /// assert_eq!(mock.last_request().unwrap().headers()["x-custom"], "1");
    /// ```
    #[must_use]
    pub fn with_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// The transport this client sends requests through.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let client = DavClient::new(MockTransport::new(), Auth::basic("a", "b"));
    /// assert!(client.transport().requests().is_empty());
    /// ```
    #[must_use]
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Build a [`RequestContext`] carrying this client's credentials, `If`
    /// precondition and extra headers.
    fn context(&self) -> RequestContext<'_> {
        RequestContext {
            auth: &self.auth,
            if_header: self.if_header.as_ref(),
            extra: &self.headers,
        }
    }

    /// Send `request` through [`DavClient::transport`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when the transport fails below the HTTP
    /// layer.
    fn send(&self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>> {
        Ok(self.transport.send(request)?)
    }

    /// `OPTIONS`: discover the compliance classes and methods `url`
    /// advertises.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Transport`] when the request fails below the HTTP layer,
    /// and [`Error::Status`] when the server does not return a success
    /// status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(200)
    ///         .header("dav", "1, 2")
    ///         .header("allow", "OPTIONS, GET, PROPFIND, LOCK")
    ///         .body(Vec::new())
    ///         .unwrap(),
    /// );
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let caps = client.options("http://localhost/").unwrap();
    /// assert!(caps.supports_locking());
    /// ```
    pub fn options(&self, url: &str) -> Result<Capabilities> {
        let request = self.context().options(url)?;
        response::capabilities(self.send(request)?)
    }

    /// `PROPFIND`: retrieve properties for `url` at the given `depth`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Xml`] when `propfind` fails to serialize or the response
    /// body fails to parse, [`Error::Transport`] when the request fails
    /// below the HTTP layer, and [`Error::Status`] when the server does not
    /// return `207 Multi-Status`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::{Depth, PropFind};
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// let body = br#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/f</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(207)
    ///         .header("content-type", "application/xml")
    ///         .body(body.to_vec())
    ///         .unwrap(),
    /// );
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let multistatus = client
    ///     .propfind("http://localhost/f", &PropFind::PropName, Depth::Zero)
    ///     .unwrap();
    /// assert_eq!(multistatus.response.len(), 1);
    /// ```
    pub fn propfind(&self, url: &str, propfind: &PropFind, depth: Depth) -> Result<Multistatus> {
        let request = self.context().propfind(url, propfind, depth)?;
        response::multistatus(self.send(request)?)
    }

    /// `PROPPATCH`: set or remove properties on `url`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Xml`] when `update` fails to serialize or the response body
    /// fails to parse, [`Error::Transport`] when the request fails below
    /// the HTTP layer, and [`Error::Status`] when the server does not
    /// return `207 Multi-Status`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::{Prop, PropertyUpdate};
    /// use dav_xml::properties::DisplayName;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// let body = br#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/f</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(207)
    ///         .header("content-type", "application/xml")
    ///         .body(body.to_vec())
    ///         .unwrap(),
    /// );
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let update = PropertyUpdate::new().remove(Prop::builder().name::<DisplayName>().build());
    /// let multistatus = client.proppatch("http://localhost/f", &update).unwrap();
    /// assert_eq!(multistatus.response.len(), 1);
    /// ```
    pub fn proppatch(&self, url: &str, update: &PropertyUpdate) -> Result<Multistatus> {
        let request = self.context().proppatch(url, update)?;
        response::multistatus(self.send(request)?)
    }

    /// `MKCOL`: create a collection at `url`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Transport`] when the request fails below the HTTP layer,
    /// and [`Error::Status`] when the server does not return a success
    /// status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(201).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// client.mkcol("http://localhost/notes/").unwrap();
    /// ```
    pub fn mkcol(&self, url: &str) -> Result<()> {
        let request = self.context().mkcol(url)?;
        response::ensure_success(self.send(request)?).map(|_response| ())
    }

    /// `GET`: retrieve `url`'s content.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Transport`] when the request fails below the HTTP layer,
    /// and [`Error::Status`] when the server does not return a success
    /// status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(200).body(b"hello".to_vec()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let response = client.get("http://localhost/notes/a.txt").unwrap();
    /// assert_eq!(response.body(), b"hello");
    /// ```
    pub fn get(&self, url: &str) -> Result<http::Response<Vec<u8>>> {
        let request = self.context().get(url)?;
        response::ensure_success(self.send(request)?)
    }

    /// `HEAD`: retrieve `url`'s headers without its content.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Transport`] when the request fails below the HTTP layer,
    /// and [`Error::Status`] when the server does not return a success
    /// status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(200)
    ///         .header("etag", "\"x\"")
    ///         .body(Vec::new())
    ///         .unwrap(),
    /// );
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let response = client.head("http://localhost/notes/a.txt").unwrap();
    /// assert_eq!(response.headers()["etag"], "\"x\"");
    /// ```
    pub fn head(&self, url: &str) -> Result<http::Response<()>> {
        let request = self.context().head(url)?;
        Ok(response::ensure_success(self.send(request)?)?.map(|_body| ()))
    }

    /// `PUT`: replace `url`'s content with `body`, sent verbatim under
    /// `content_type`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Transport`] when the request fails below the HTTP layer,
    /// and [`Error::Status`] when the server does not return a success
    /// status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// client
    ///     .put("http://localhost/notes/a.txt", b"hi".to_vec(), "text/plain")
    ///     .unwrap();
    /// ```
    pub fn put(&self, url: &str, body: Vec<u8>, content_type: &str) -> Result<()> {
        let request = self.context().put(url, body, content_type)?;
        response::ensure_success(self.send(request)?).map(|_response| ())
    }

    /// `DELETE`: remove `url`.
    ///
    /// A `207 Multi-Status` response reporting at least one failure surfaces
    /// as [`Error::Multistatus`] rather than [`Error::Status`], since the
    /// overall request still succeeded.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Transport`] when the request fails below the HTTP layer,
    /// [`Error::Status`] when the server does not return a success status,
    /// and [`Error::Multistatus`] when a `207` body reports at least one
    /// failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// client.delete("http://localhost/notes/a.txt").unwrap();
    /// ```
    pub fn delete(&self, url: &str) -> Result<()> {
        let request = self.context().delete(url)?;
        response::ok_or_multistatus(self.send(request)?)
    }

    /// `COPY`: duplicate `url` to `destination`.
    ///
    /// A `207 Multi-Status` response reporting at least one failure surfaces
    /// as [`Error::Multistatus`] rather than [`Error::Status`], since the
    /// overall request still succeeded.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` or `destination` is not a
    /// valid URI, [`Error::InvalidArgument`] when `depth` is [`Depth::One`],
    /// which `COPY` does not permit, [`Error::Transport`] when the request
    /// fails below the HTTP layer, [`Error::Status`] when the server does
    /// not return a success status, and [`Error::Multistatus`] when a `207`
    /// body reports at least one failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::Depth;
    /// use dav_xml_client::headers::Overwrite;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(201).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// client
    ///     .copy(
    ///         "http://localhost/a.txt",
    ///         "http://localhost/b.txt",
    ///         Depth::Infinity,
    ///         Overwrite::False,
    ///     )
    ///     .unwrap();
    /// ```
    pub fn copy(
        &self,
        url: &str,
        destination: &str,
        depth: Depth,
        overwrite: Overwrite,
    ) -> Result<()> {
        let request = self.context().copy(url, destination, depth, overwrite)?;
        response::ok_or_multistatus(self.send(request)?)
    }

    /// `MOVE`: relocate `url` to `destination`.
    ///
    /// A `207 Multi-Status` response reporting at least one failure surfaces
    /// as [`Error::Multistatus`] rather than [`Error::Status`], since the
    /// overall request still succeeded.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` or `destination` is not a
    /// valid URI, [`Error::Transport`] when the request fails below the
    /// HTTP layer, [`Error::Status`] when the server does not return a
    /// success status, and [`Error::Multistatus`] when a `207` body reports
    /// at least one failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::headers::Overwrite;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// client
    ///     .mv("http://localhost/a.txt", "http://localhost/b.txt", Overwrite::True)
    ///     .unwrap();
    /// ```
    pub fn mv(&self, url: &str, destination: &str, overwrite: Overwrite) -> Result<()> {
        let request = self.context().mv(url, destination, overwrite)?;
        response::ok_or_multistatus(self.send(request)?)
    }

    /// `LOCK`: create a new lock on `url`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::InvalidArgument`] when `depth` is [`Depth::One`], which
    /// `LOCK` does not permit, [`Error::Xml`] when `info` fails to
    /// serialize or the response body fails to parse, [`Error::Transport`]
    /// when the request fails below the HTTP layer, [`Error::Status`] when
    /// the server does not return a success status, and
    /// [`Error::MissingLockToken`] when neither the `Lock-Token` header nor
    /// the `lockdiscovery` body carries a token.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::{Depth, LockInfo};
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// let body = br#"<D:prop xmlns:D="DAV:"><D:lockdiscovery><D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>0</D:depth><D:locktoken><D:href>urn:uuid:1</D:href></D:locktoken><D:lockroot><D:href>/f</D:href></D:lockroot></D:activelock></D:lockdiscovery></D:prop>"#;
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(200)
    ///         .header("content-type", "application/xml")
    ///         .header("lock-token", "<urn:uuid:1>")
    ///         .body(body.to_vec())
    ///         .unwrap(),
    /// );
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let lock = client
    ///     .lock("http://localhost/f", &LockInfo::exclusive_write(), None, Depth::Zero)
    ///     .unwrap();
    /// assert_eq!(lock.token.0, "urn:uuid:1");
    /// ```
    pub fn lock(
        &self,
        url: &str,
        info: &LockInfo,
        timeout: Option<Timeout>,
        depth: Depth,
    ) -> Result<Lock> {
        let request = self.context().lock(url, info, timeout, depth)?;
        let (discovery, token) = response::lock_discovery(self.send(request)?)?;
        response::lock_result(discovery, token, None)
    }

    /// `LOCK` with an `If` header carrying `token`: refresh an existing
    /// lock's timeout.
    ///
    /// The returned [`Lock::token`] keeps `token` when the server's
    /// response carries neither a `Lock-Token` header nor a `lockdiscovery`
    /// token of its own.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::InvalidHeader`] when the merged `If` header renders to an
    /// invalid header value, [`Error::Transport`] when the request fails
    /// below the HTTP layer, [`Error::Status`] when the server does not
    /// return a success status, and [`Error::Xml`] when the response body
    /// fails to parse.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml::elements::Timeout;
    /// use dav_xml_client::headers::LockTokenHeader;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// let body = br#"<D:prop xmlns:D="DAV:"><D:lockdiscovery><D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>0</D:depth><D:locktoken><D:href>urn:uuid:1</D:href></D:locktoken><D:lockroot><D:href>/f</D:href></D:lockroot></D:activelock></D:lockdiscovery></D:prop>"#;
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(200)
    ///         .header("content-type", "application/xml")
    ///         .body(body.to_vec())
    ///         .unwrap(),
    /// );
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let token = LockTokenHeader("urn:uuid:1".to_owned());
    /// let lock = client
    ///     .refresh_lock("http://localhost/f", &token, Some(Timeout::Seconds(120)))
    ///     .unwrap();
    /// assert_eq!(lock.token.0, "urn:uuid:1");
    /// ```
    pub fn refresh_lock(
        &self,
        url: &str,
        token: &LockTokenHeader,
        timeout: Option<Timeout>,
    ) -> Result<Lock> {
        let request = self.context().refresh_lock(url, token, timeout)?;
        let (discovery, header) = response::lock_discovery(self.send(request)?)?;
        response::lock_result(discovery, header, Some(token.clone()))
    }

    /// `UNLOCK`: remove the lock identified by `token` from `url`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidUrl`] when `url` is not a valid URI,
    /// [`Error::Transport`] when the request fails below the HTTP layer,
    /// and [`Error::Status`] when the server does not return a success
    /// status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::headers::LockTokenHeader;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let token = LockTokenHeader("urn:uuid:1".to_owned());
    /// client.unlock("http://localhost/f", &token).unwrap();
    /// ```
    pub fn unlock(&self, url: &str, token: &LockTokenHeader) -> Result<()> {
        let request = self.context().unlock(url, token)?;
        response::ensure_success(self.send(request)?).map(|_response| ())
    }

    /// List the immediate children of the collection at `url`.
    ///
    /// A convenience over [`DavClient::propfind`] requesting every standard
    /// property (`allprop`) at [`Depth::One`], flattened through
    /// [`Resource::from_multistatus`].
    ///
    /// # Errors
    ///
    /// See [`DavClient::propfind`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// let body = br#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/dir/f</D:href><D:propstat><D:prop/><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(207)
    ///         .header("content-type", "application/xml")
    ///         .body(body.to_vec())
    ///         .unwrap(),
    /// );
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// let resources = client.list("http://localhost/dir/").unwrap();
    /// assert_eq!(resources[0].path(), "/dir/f");
    /// ```
    pub fn list(&self, url: &str) -> Result<Vec<Resource>> {
        let multistatus = self.propfind(url, &PropFind::AllProp { include: None }, Depth::One)?;
        Ok(Resource::from_multistatus(&multistatus))
    }

    /// Read the properties of `url` itself, without listing its children.
    ///
    /// A convenience over [`DavClient::propfind`] requesting every standard
    /// property (`allprop`) at [`Depth::Zero`]. A `404 Not Found` response
    /// is reported as [`None`] rather than an error.
    ///
    /// # Errors
    ///
    /// See [`DavClient::propfind`], except for a `404` status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(404).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// assert!(client.stat("http://localhost/missing").unwrap().is_none());
    /// ```
    pub fn stat(&self, url: &str) -> Result<Option<Resource>> {
        match self.propfind(url, &PropFind::AllProp { include: None }, Depth::Zero) {
            Ok(multistatus) => Ok(Resource::from_multistatus(&multistatus).into_iter().next()),
            Err(Error::Status {
                status: http::StatusCode::NOT_FOUND,
                ..
            }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Whether `url` exists.
    ///
    /// A convenience over [`DavClient::head`]. A `404 Not Found` response is
    /// reported as `false` rather than an error.
    ///
    /// # Errors
    ///
    /// See [`DavClient::head`], except for a `404` status.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(404).body(Vec::new()).unwrap());
    /// let client = DavClient::new(mock, Auth::basic("a", "b"));
    /// assert!(!client.exists("http://localhost/missing").unwrap());
    /// ```
    pub fn exists(&self, url: &str) -> Result<bool> {
        match self.head(url) {
            Ok(_response) => Ok(true),
            Err(Error::Status {
                status: http::StatusCode::NOT_FOUND,
                ..
            }) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

#[cfg(feature = "ureq")]
impl DavClient<ureq::Agent> {
    /// A client over [`crate::transport::ureq::default_agent`], which
    /// returns error statuses as responses and follows no redirects.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let client = DavClient::ureq(Auth::basic("alice", "secret"));
    /// assert!(!client.transport().config().http_status_as_error());
    /// ```
    #[must_use]
    pub fn ureq(auth: Auth) -> Self {
        Self::new(crate::transport::ureq::default_agent(), auth)
    }

    /// A client over a caller-configured [`ureq::Agent`].
    ///
    /// Configure `agent` with `http_status_as_error(false)` (see
    /// [`ureq::config::ConfigBuilder::http_status_as_error`]), or every
    /// `4xx`/`5xx` response becomes a transport error instead of a value
    /// this client can inspect.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::ureq::default_agent;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let client = DavClient::ureq_with(default_agent(), Auth::basic("alice", "secret"));
    /// assert!(!client.transport().config().http_status_as_error());
    /// ```
    #[must_use]
    pub fn ureq_with(agent: ureq::Agent, auth: Auth) -> Self {
        Self::new(agent, auth)
    }
}

#[cfg(feature = "isahc")]
impl DavClient<isahc::HttpClient> {
    /// A client over [`crate::transport::isahc::default_client`], which
    /// follows no redirects.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when libcurl cannot be initialised.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let client = DavClient::isahc(Auth::basic("alice", "secret")).unwrap();
    /// let exists = client.exists("https://example.com/file").unwrap();
    /// # let _ = exists;
    /// ```
    pub fn isahc(auth: Auth) -> Result<Self> {
        Ok(Self::new(crate::transport::isahc::default_client()?, auth))
    }

    /// A client over a caller-configured [`isahc::HttpClient`].
    ///
    /// `isahc`'s default redirect policy already follows no redirects, but
    /// build `client` accordingly if you have overridden it, or `3xx`
    /// responses are resolved by `isahc` instead of reaching this client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dav_xml_client::transport::isahc::default_client;
    /// use dav_xml_client::{Auth, DavClient};
    ///
    /// let client = DavClient::isahc_with(default_client().unwrap(), Auth::basic("alice", "secret"));
    /// let exists = client.exists("https://example.com/file").unwrap();
    /// # let _ = exists;
    /// ```
    #[must_use]
    pub fn isahc_with(client: isahc::HttpClient, auth: Auth) -> Self {
        Self::new(client, auth)
    }
}
