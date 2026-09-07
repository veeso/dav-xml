// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The async `WebDAV` client.

#[cfg(test)]
mod tests {
    use dav_xml::elements::Prop;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::transport::MockTransport;

    fn client() -> (AsyncDavClient<MockTransport>, MockTransport) {
        let mock = MockTransport::new();
        (
            AsyncDavClient::new(mock.clone(), Auth::basic("a", "b")),
            mock,
        )
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

    #[tokio::test]
    async fn list_sends_allprop_depth_one_and_flattens() {
        let (client, mock) = client();
        reply(&mock, 207, LISTING);
        let resources = client.list("http://h/d/").await.unwrap();
        let request = mock.last_request().unwrap();
        assert_eq!(request.method().as_str(), "PROPFIND");
        assert_eq!(request.headers()["depth"], "1");
        assert_eq!(resources.len(), 2);
        assert!(resources[0].is_collection);
    }

    #[tokio::test]
    async fn stat_returns_none_on_404() {
        let (client, mock) = client();
        reply(&mock, 404, "");
        assert!(client.stat("http://h/x").await.unwrap().is_none());
        reply(&mock, 207, LISTING);
        assert!(client.stat("http://h/d/").await.unwrap().is_some());
        assert_eq!(mock.last_request().unwrap().headers()["depth"], "0");
    }

    #[tokio::test]
    async fn exists_uses_head() {
        let (client, mock) = client();
        reply(&mock, 200, "");
        assert!(client.exists("http://h/f").await.unwrap());
        assert_eq!(mock.last_request().unwrap().method(), http::Method::HEAD);
        reply(&mock, 404, "");
        assert!(!client.exists("http://h/f").await.unwrap());
    }

    #[tokio::test]
    async fn propfind_returns_multistatus() {
        let (client, mock) = client();
        reply(&mock, 207, LISTING);
        let ms = client
            .propfind("http://h/d/", &PropFind::PropName, Depth::Zero)
            .await
            .unwrap();
        assert_eq!(ms.response.len(), 2);
    }

    #[tokio::test]
    async fn proppatch_returns_multistatus() {
        let (client, mock) = client();
        reply(&mock, 207, LISTING);
        let update = PropertyUpdate::new().remove(
            Prop::builder()
                .name::<dav_xml::properties::DisplayName>()
                .build(),
        );
        let ms = client.proppatch("http://h/d/", &update).await.unwrap();
        assert_eq!(ms.response.len(), 2);
        assert_eq!(mock.last_request().unwrap().method().as_str(), "PROPPATCH");
    }

    #[tokio::test]
    async fn mkcol_put_delete_return_unit_on_2xx() {
        let (client, mock) = client();
        reply(&mock, 201, "");
        client.mkcol("http://h/d/").await.unwrap();
        reply(&mock, 204, "");
        client
            .put("http://h/d/f", b"x".to_vec(), "text/plain")
            .await
            .unwrap();
        reply(&mock, 204, "");
        client.delete("http://h/d/f").await.unwrap();
        assert_eq!(mock.requests().len(), 3);
        assert_eq!(client.transport().requests().len(), 3);
    }

    #[tokio::test]
    async fn delete_207_with_failures_is_an_error() {
        let (client, mock) = client();
        reply(
            &mock,
            207,
            r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/d/f</D:href><D:status>HTTP/1.1 423 Locked</D:status></D:response></D:multistatus>"#,
        );
        assert!(matches!(
            client.delete("http://h/d/").await,
            Err(Error::Multistatus(_))
        ));
    }

    #[tokio::test]
    async fn copy_and_move_forward_headers() {
        let (client, mock) = client();
        reply(&mock, 201, "");
        client
            .copy(
                "http://h/a",
                "http://h/b",
                Depth::Infinity,
                Overwrite::False,
            )
            .await
            .unwrap();
        assert_eq!(mock.last_request().unwrap().headers()["overwrite"], "F");
        reply(&mock, 204, "");
        client
            .mv("http://h/a", "http://h/c", Overwrite::True)
            .await
            .unwrap();
        assert_eq!(
            mock.last_request().unwrap().headers()["destination"],
            "http://h/c"
        );
    }

    #[tokio::test]
    async fn get_and_head_return_responses() {
        let (client, mock) = client();
        reply(&mock, 200, "");
        mock.reply(
            http::Response::builder()
                .status(200)
                .header("etag", "\"x\"")
                .body(b"body".to_vec())
                .unwrap(),
        );
        let head = client.head("http://h/f").await.unwrap();
        assert_eq!(head.status(), 200);
        let get = client.get("http://h/f").await.unwrap();
        assert_eq!(get.body(), b"body");
        assert_eq!(get.headers()["etag"], "\"x\"");
    }

    #[tokio::test]
    async fn lock_refresh_unlock_flow() {
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
            .await
            .unwrap();
        assert_eq!(lock.token.0, "urn:uuid:1");
        assert_eq!(lock.discovery.0[0].timeout, Some(Timeout::Seconds(60)));

        reply(&mock, 200, LOCK_BODY);
        let refreshed = client
            .refresh_lock("http://h/d/f", &lock.token, Some(Timeout::Seconds(120)))
            .await
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
        client.unlock("http://h/d/f", &lock.token).await.unwrap();
        assert_eq!(
            mock.last_request().unwrap().headers()["lock-token"],
            "<urn:uuid:1>"
        );
    }

    #[tokio::test]
    async fn lock_without_token_header_or_body_token_is_an_error() {
        let (client, mock) = client();
        reply(
            &mock,
            200,
            r#"<D:prop xmlns:D="DAV:"><D:lockdiscovery/></D:prop>"#,
        );
        assert!(matches!(
            client
                .lock(
                    "http://h/f",
                    &LockInfo::exclusive_write(),
                    None,
                    Depth::Zero
                )
                .await,
            Err(Error::MissingLockToken)
        ));
    }

    #[tokio::test]
    async fn with_if_applies_to_every_request() {
        let (client, mock) = client();
        let client = client.with_if(If::untagged("urn:uuid:1"));
        reply(&mock, 204, "");
        client
            .put("http://h/f", Vec::new(), "text/plain")
            .await
            .unwrap();
        assert_eq!(
            mock.last_request().unwrap().headers()["if"],
            "(<urn:uuid:1>)"
        );
    }

    #[tokio::test]
    async fn with_header_sets_custom_header() {
        let (client, mock) = client();
        let client = client.with_header(
            http::HeaderName::from_static("x-custom"),
            http::HeaderValue::from_static("1"),
        );
        reply(&mock, 204, "");
        client
            .put("http://h/f", Vec::new(), "text/plain")
            .await
            .unwrap();
        assert_eq!(mock.last_request().unwrap().headers()["x-custom"], "1");
    }

    #[tokio::test]
    async fn status_errors_carry_dav_error() {
        let (client, mock) = client();
        reply(
            &mock,
            423,
            r#"<D:error xmlns:D="DAV:"><D:lock-token-submitted><D:href>/f</D:href></D:lock-token-submitted></D:error>"#,
        );
        let error = client
            .put("http://h/f", Vec::new(), "text/plain")
            .await
            .unwrap_err();
        assert_eq!(error.status().unwrap(), 423);
        assert!(error.dav_error().unwrap().contains("lock-token-submitted"));
    }

    #[tokio::test]
    async fn transport_errors_are_wrapped() {
        let (client, _mock) = client();
        assert!(matches!(
            client.get("http://h/f").await,
            Err(Error::Transport(_))
        ));
    }

    #[tokio::test]
    async fn options_returns_capabilities() {
        let (client, mock) = client();
        mock.reply(
            http::Response::builder()
                .status(200)
                .header("dav", "1,2")
                .header("allow", "GET")
                .body(Vec::new())
                .unwrap(),
        );
        let caps = client.options("http://h/").await.unwrap();
        assert!(caps.supports_locking());
    }

    #[test]
    fn client_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AsyncDavClient<MockTransport>>();
    }

    #[tokio::test]
    async fn futures_are_send() {
        fn assert_send<F: Send>(_: &F) {}
        let (client, mock) = client();
        reply(&mock, 204, "");
        let future = client.mkcol("http://h/d/");
        assert_send(&future);
        future.await.unwrap();
    }
}

use dav_xml::elements::{Depth, LockInfo, Multistatus, PropFind, PropertyUpdate, Timeout};
use http::{HeaderName, HeaderValue};

use crate::capabilities::Capabilities;
use crate::client::Lock;
use crate::error::{Error, Result};
use crate::headers::{If, LockTokenHeader, Overwrite};
use crate::request::RequestContext;
use crate::resource::Resource;
use crate::transport::AsyncTransport;
use crate::{Auth, response};

/// An async `WebDAV` client.
///
/// Every method builds one request, sends it through `T`, and interprets
/// the response into a typed result or an [`Error`]. A precondition set
/// with [`AsyncDavClient::with_if`] and any header set with
/// [`AsyncDavClient::with_header`] apply to every request the client sends.
///
/// Because building a request only borrows `self` for the duration of one
/// synchronous call, every method's future stays [`Send`] as long as `T:
/// Sync`.
///
/// # Examples
///
/// ```
/// # #[tokio::main]
/// # async fn main() {
/// use dav_xml_client::transport::MockTransport;
/// use dav_xml_client::{AsyncDavClient, Auth};
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
/// let client = AsyncDavClient::new(mock, Auth::basic("alice", "secret"));
/// for resource in client.list("http://localhost:3080/").await.unwrap() {
///     println!("{path}", path = resource.path());
/// }
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct AsyncDavClient<T> {
    transport: T,
    auth: Auth,
    if_header: Option<If>,
    headers: http::HeaderMap,
}

impl<T: AsyncTransport> AsyncDavClient<T> {
    /// Build a client sending every request through `transport`, using
    /// `auth` for the `Authorization` header.
    ///
    /// # Examples
    ///
    /// ```
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let client = AsyncDavClient::new(MockTransport::new(), Auth::basic("alice", "secret"));
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
    /// sends afterward (a lock token acquired from [`AsyncDavClient::lock`],
    /// for example).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::headers::If;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock.clone(), Auth::basic("a", "b"))
    ///     .with_if(If::untagged("urn:uuid:1"));
    /// client
    ///     .put("http://localhost/f", Vec::new(), "text/plain")
    ///     .await
    ///     .unwrap();
    /// assert_eq!(mock.last_request().unwrap().headers()["if"], "(<urn:uuid:1>)");
    /// # }
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock.clone(), Auth::basic("a", "b")).with_header(
    ///     http::HeaderName::from_static("x-custom"),
    ///     http::HeaderValue::from_static("1"),
    /// );
    /// client
    ///     .put("http://localhost/f", Vec::new(), "text/plain")
    ///     .await
    ///     .unwrap();
    /// assert_eq!(mock.last_request().unwrap().headers()["x-custom"], "1");
    /// # }
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
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let client = AsyncDavClient::new(MockTransport::new(), Auth::basic("a", "b"));
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

    /// Send `request` through [`AsyncDavClient::transport`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] when the transport fails below the HTTP
    /// layer.
    async fn send(&self, request: http::Request<Vec<u8>>) -> Result<http::Response<Vec<u8>>> {
        Ok(self.transport.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
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
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let caps = client.options("http://localhost/").await.unwrap();
    /// assert!(caps.supports_locking());
    /// # }
    /// ```
    pub async fn options(&self, url: &str) -> Result<Capabilities> {
        let request = self.context().options(url)?;
        response::capabilities(self.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml::elements::{Depth, PropFind};
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
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
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let multistatus = client
    ///     .propfind("http://localhost/f", &PropFind::PropName, Depth::Zero)
    ///     .await
    ///     .unwrap();
    /// assert_eq!(multistatus.response.len(), 1);
    /// # }
    /// ```
    pub async fn propfind(
        &self,
        url: &str,
        propfind: &PropFind,
        depth: Depth,
    ) -> Result<Multistatus> {
        let request = self.context().propfind(url, propfind, depth)?;
        response::multistatus(self.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml::elements::{Prop, PropertyUpdate};
    /// use dav_xml::properties::DisplayName;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
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
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let update = PropertyUpdate::new().remove(Prop::builder().name::<DisplayName>().build());
    /// let multistatus = client
    ///     .proppatch("http://localhost/f", &update)
    ///     .await
    ///     .unwrap();
    /// assert_eq!(multistatus.response.len(), 1);
    /// # }
    /// ```
    pub async fn proppatch(&self, url: &str, update: &PropertyUpdate) -> Result<Multistatus> {
        let request = self.context().proppatch(url, update)?;
        response::multistatus(self.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(201).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// client.mkcol("http://localhost/notes/").await.unwrap();
    /// # }
    /// ```
    pub async fn mkcol(&self, url: &str) -> Result<()> {
        let request = self.context().mkcol(url)?;
        response::ensure_success(self.send(request).await?).map(|_response| ())
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(200).body(b"hello".to_vec()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let response = client.get("http://localhost/notes/a.txt").await.unwrap();
    /// assert_eq!(response.body(), b"hello");
    /// # }
    /// ```
    pub async fn get(&self, url: &str) -> Result<http::Response<Vec<u8>>> {
        let request = self.context().get(url)?;
        response::ensure_success(self.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(
    ///     http::Response::builder()
    ///         .status(200)
    ///         .header("etag", "\"x\"")
    ///         .body(Vec::new())
    ///         .unwrap(),
    /// );
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let response = client.head("http://localhost/notes/a.txt").await.unwrap();
    /// assert_eq!(response.headers()["etag"], "\"x\"");
    /// # }
    /// ```
    pub async fn head(&self, url: &str) -> Result<http::Response<()>> {
        let request = self.context().head(url)?;
        Ok(response::ensure_success(self.send(request).await?)?.map(|_body| ()))
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// client
    ///     .put("http://localhost/notes/a.txt", b"hi".to_vec(), "text/plain")
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub async fn put(&self, url: &str, body: Vec<u8>, content_type: &str) -> Result<()> {
        let request = self.context().put(url, body, content_type)?;
        response::ensure_success(self.send(request).await?).map(|_response| ())
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// client.delete("http://localhost/notes/a.txt").await.unwrap();
    /// # }
    /// ```
    pub async fn delete(&self, url: &str) -> Result<()> {
        let request = self.context().delete(url)?;
        response::ok_or_multistatus(self.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml::elements::Depth;
    /// use dav_xml_client::headers::Overwrite;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(201).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// client
    ///     .copy(
    ///         "http://localhost/a.txt",
    ///         "http://localhost/b.txt",
    ///         Depth::Infinity,
    ///         Overwrite::False,
    ///     )
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub async fn copy(
        &self,
        url: &str,
        destination: &str,
        depth: Depth,
        overwrite: Overwrite,
    ) -> Result<()> {
        let request = self.context().copy(url, destination, depth, overwrite)?;
        response::ok_or_multistatus(self.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::headers::Overwrite;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// client
    ///     .mv("http://localhost/a.txt", "http://localhost/b.txt", Overwrite::True)
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub async fn mv(&self, url: &str, destination: &str, overwrite: Overwrite) -> Result<()> {
        let request = self.context().mv(url, destination, overwrite)?;
        response::ok_or_multistatus(self.send(request).await?)
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml::elements::{Depth, LockInfo};
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
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
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let lock = client
    ///     .lock("http://localhost/f", &LockInfo::exclusive_write(), None, Depth::Zero)
    ///     .await
    ///     .unwrap();
    /// assert_eq!(lock.token.0, "urn:uuid:1");
    /// # }
    /// ```
    pub async fn lock(
        &self,
        url: &str,
        info: &LockInfo,
        timeout: Option<Timeout>,
        depth: Depth,
    ) -> Result<Lock> {
        let request = self.context().lock(url, info, timeout, depth)?;
        let (discovery, token) = response::lock_discovery(self.send(request).await?)?;
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml::elements::Timeout;
    /// use dav_xml_client::headers::LockTokenHeader;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
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
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let token = LockTokenHeader("urn:uuid:1".to_owned());
    /// let lock = client
    ///     .refresh_lock("http://localhost/f", &token, Some(Timeout::Seconds(120)))
    ///     .await
    ///     .unwrap();
    /// assert_eq!(lock.token.0, "urn:uuid:1");
    /// # }
    /// ```
    pub async fn refresh_lock(
        &self,
        url: &str,
        token: &LockTokenHeader,
        timeout: Option<Timeout>,
    ) -> Result<Lock> {
        let request = self.context().refresh_lock(url, token, timeout)?;
        let (discovery, header) = response::lock_discovery(self.send(request).await?)?;
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
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::headers::LockTokenHeader;
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let token = LockTokenHeader("urn:uuid:1".to_owned());
    /// client.unlock("http://localhost/f", &token).await.unwrap();
    /// # }
    /// ```
    pub async fn unlock(&self, url: &str, token: &LockTokenHeader) -> Result<()> {
        let request = self.context().unlock(url, token)?;
        response::ensure_success(self.send(request).await?).map(|_response| ())
    }

    /// List the immediate children of the collection at `url`.
    ///
    /// A convenience over [`AsyncDavClient::propfind`] requesting every
    /// standard property (`allprop`) at [`Depth::One`], flattened through
    /// [`Resource::from_multistatus`].
    ///
    /// # Errors
    ///
    /// See [`AsyncDavClient::propfind`].
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
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
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// let resources = client.list("http://localhost/dir/").await.unwrap();
    /// assert_eq!(resources[0].path(), "/dir/f");
    /// # }
    /// ```
    pub async fn list(&self, url: &str) -> Result<Vec<Resource>> {
        let multistatus = self
            .propfind(url, &PropFind::AllProp { include: None }, Depth::One)
            .await?;
        Ok(Resource::from_multistatus(&multistatus))
    }

    /// Read the properties of `url` itself, without listing its children.
    ///
    /// A convenience over [`AsyncDavClient::propfind`] requesting every
    /// standard property (`allprop`) at [`Depth::Zero`]. A `404 Not Found`
    /// response is reported as [`None`] rather than an error.
    ///
    /// # Errors
    ///
    /// See [`AsyncDavClient::propfind`], except for a `404` status.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(404).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// assert!(client.stat("http://localhost/missing").await.unwrap().is_none());
    /// # }
    /// ```
    pub async fn stat(&self, url: &str) -> Result<Option<Resource>> {
        match self
            .propfind(url, &PropFind::AllProp { include: None }, Depth::Zero)
            .await
        {
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
    /// A convenience over [`AsyncDavClient::head`]. A `404 Not Found`
    /// response is reported as `false` rather than an error.
    ///
    /// # Errors
    ///
    /// See [`AsyncDavClient::head`], except for a `404` status.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::MockTransport;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let mock = MockTransport::new();
    /// mock.reply(http::Response::builder().status(404).body(Vec::new()).unwrap());
    /// let client = AsyncDavClient::new(mock, Auth::basic("a", "b"));
    /// assert!(!client.exists("http://localhost/missing").await.unwrap());
    /// # }
    /// ```
    pub async fn exists(&self, url: &str) -> Result<bool> {
        match self.head(url).await {
            Ok(_response) => Ok(true),
            Err(Error::Status {
                status: http::StatusCode::NOT_FOUND,
                ..
            }) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

#[cfg(feature = "reqwest")]
impl AsyncDavClient<reqwest::Client> {
    /// A client over [`crate::transport::reqwest::default_client`], which
    /// follows no redirects.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let client = AsyncDavClient::reqwest(Auth::basic("alice", "secret"));
    /// let exists = client.exists("https://example.com/file").await.unwrap();
    /// # let _ = exists;
    /// # }
    /// ```
    #[must_use]
    pub fn reqwest(auth: Auth) -> Self {
        Self::new(crate::transport::reqwest::default_client(), auth)
    }

    /// A client over a caller-configured [`reqwest::Client`].
    ///
    /// `reqwest`'s default [`reqwest::redirect::Policy`] follows redirects;
    /// build `client` with [`reqwest::redirect::Policy::none()`] (as
    /// [`crate::transport::reqwest::default_client`] does), or `3xx`
    /// responses are resolved by `reqwest` instead of reaching this client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() {
    /// use dav_xml_client::transport::reqwest::default_client;
    /// use dav_xml_client::{AsyncDavClient, Auth};
    ///
    /// let client = AsyncDavClient::reqwest_with(default_client(), Auth::basic("alice", "secret"));
    /// let exists = client.exists("https://example.com/file").await.unwrap();
    /// # let _ = exists;
    /// # }
    /// ```
    #[must_use]
    pub fn reqwest_with(client: reqwest::Client, auth: Auth) -> Self {
        Self::new(client, auth)
    }
}
