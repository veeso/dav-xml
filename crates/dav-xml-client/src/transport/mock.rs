// SPDX-FileCopyrightText: Christian Visintin <christian.visintin@veeso.dev>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::{AsyncTransport, Transport, TransportError, TransportErrorKind};

/// A scripted transport for tests: replies are returned in FIFO order and
/// every request is recorded.
///
/// Available in tests and behind the `mock` feature.
///
/// # Examples
///
/// ```
/// use dav_xml_client::transport::{MockTransport, Transport};
///
/// let mock = MockTransport::new();
/// mock.reply(http::Response::builder().status(204).body(Vec::new()).unwrap());
/// let request = http::Request::builder().uri("http://x/").body(Vec::new()).unwrap();
/// assert_eq!(mock.send(request).unwrap().status(), 204);
/// assert_eq!(mock.requests().len(), 1);
/// ```
#[derive(Clone, Debug, Default)]
pub struct MockTransport {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    replies: VecDeque<http::Response<Vec<u8>>>,
    requests: Vec<http::Request<Vec<u8>>>,
}

impl MockTransport {
    /// An empty mock.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a reply.
    pub fn reply(&self, response: http::Response<Vec<u8>>) {
        self.lock().replies.push_back(response);
    }

    /// Every request sent so far, oldest first.
    #[must_use]
    pub fn requests(&self) -> Vec<http::Request<Vec<u8>>> {
        self.lock().requests.iter().map(clone_request).collect()
    }

    /// The last request sent.
    #[must_use]
    pub fn last_request(&self) -> Option<http::Request<Vec<u8>>> {
        self.lock().requests.last().map(clone_request)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn take(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        let mut inner = self.lock();
        inner.requests.push(request);
        inner.replies.pop_front().ok_or_else(|| {
            TransportError::new(TransportErrorKind::Other, "mock transport: no reply queued")
        })
    }
}

fn clone_request(request: &http::Request<Vec<u8>>) -> http::Request<Vec<u8>> {
    let mut builder = http::Request::builder()
        .method(request.method())
        .uri(request.uri());
    for (name, value) in request.headers() {
        builder = builder.header(name, value);
    }
    builder
        .body(request.body().clone())
        .unwrap_or_else(|_| http::Request::new(Vec::new()))
}

impl Transport for MockTransport {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, TransportError> {
        self.take(request)
    }
}

impl AsyncTransport for MockTransport {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> impl std::future::Future<Output = Result<http::Response<Vec<u8>>, TransportError>> + Send
    {
        std::future::ready(self.take(request))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn request() -> http::Request<Vec<u8>> {
        http::Request::builder()
            .method("GET")
            .uri("http://x/a")
            .body(Vec::new())
            .unwrap()
    }

    #[test]
    fn replies_in_order_and_records_requests() {
        let mock = MockTransport::new();
        mock.reply(
            http::Response::builder()
                .status(200)
                .body(b"one".to_vec())
                .unwrap(),
        );
        mock.reply(
            http::Response::builder()
                .status(404)
                .body(Vec::new())
                .unwrap(),
        );
        assert_eq!(Transport::send(&mock, request()).unwrap().status(), 200);
        assert_eq!(Transport::send(&mock, request()).unwrap().status(), 404);
        assert_eq!(mock.requests().len(), 2);
    }

    #[test]
    fn exhausted_replies_is_a_transport_error() {
        let mock = MockTransport::new();
        let error = Transport::send(&mock, request()).unwrap_err();
        assert_eq!(error.kind(), TransportErrorKind::Other);
    }

    #[tokio::test]
    async fn async_send_shares_the_queue() {
        let mock = MockTransport::new();
        mock.reply(
            http::Response::builder()
                .status(204)
                .body(Vec::new())
                .unwrap(),
        );
        let response = AsyncTransport::send(&mock, request()).await.unwrap();
        assert_eq!(response.status(), 204);
    }
}
