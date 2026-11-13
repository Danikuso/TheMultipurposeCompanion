//! Transportes simulados para tests (nunca tocan la red).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::transport::{BoxFut, HttpPost, HttpReply, HttpReplyEx, Transport, TransportError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedPost {
    pub url: String,
    pub content_type: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub http1_only: bool,
}

/// Transporte con respuestas FIFO preprogramadas que registra los POST y GET.
pub struct ScriptedTransport {
    replies: Mutex<VecDeque<Result<HttpReplyEx, TransportError>>>,
    get_replies: Mutex<VecDeque<Result<HttpReply, TransportError>>>,
    posts: Mutex<Vec<RecordedPost>>,
    get_urls: Mutex<Vec<String>>,
    calls: AtomicUsize,
}

impl ScriptedTransport {
    pub fn new(replies: Vec<Result<HttpReplyEx, TransportError>>) -> Arc<Self> {
        Arc::new(Self {
            replies: Mutex::new(replies.into()),
            get_replies: Mutex::new(VecDeque::new()),
            posts: Mutex::new(Vec::new()),
            get_urls: Mutex::new(Vec::new()),
            calls: AtomicUsize::new(0),
        })
    }

    pub fn posts(&self) -> Vec<RecordedPost> {
        self.posts.lock().unwrap().clone()
    }

    pub fn get_urls(&self) -> Vec<String> {
        self.get_urls.lock().unwrap().clone()
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub fn push(&self, r: Result<HttpReplyEx, TransportError>) {
        self.replies.lock().unwrap().push_back(r);
    }

    pub fn push_get(&self, r: Result<HttpReply, TransportError>) {
        self.get_replies.lock().unwrap().push_back(r);
    }

    fn unsupported<T: Send + 'static>(&self) -> BoxFut<'_, Result<T, TransportError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Err(TransportError::Connect(
                "no soportado en ScriptedTransport".into(),
            ))
        })
    }
}

impl Transport for ScriptedTransport {
    fn get(&self, url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        let next = self.get_replies.lock().unwrap().pop_front();
        match next {
            Some(reply) => {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.get_urls.lock().unwrap().push(url.to_owned());
                Box::pin(async move { reply })
            }
            None => self.unsupported(),
        }
    }

    fn post_form(
        &self,
        _url: &str,
        _form: &[(String, String)],
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.unsupported()
    }

    fn post<'a>(&'a self, req: HttpPost<'a>) -> BoxFut<'a, Result<HttpReplyEx, TransportError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.posts.lock().unwrap().push(RecordedPost {
            url: req.url.to_owned(),
            content_type: req.content_type.to_owned(),
            headers: req
                .headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            body: req.body.to_vec(),
            http1_only: req.http1_only,
        });
        let next = self.replies.lock().unwrap().pop_front();
        Box::pin(async move {
            next.unwrap_or_else(|| Err(TransportError::Connect("sin respuestas".into())))
        })
    }
}

/// Transporte que falla siempre como "sin red" y cuenta las llamadas.
#[derive(Default)]
pub struct NoNetworkTransport {
    calls: AtomicUsize,
}

impl NoNetworkTransport {
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn fail<T: Send + 'static>(&self) -> BoxFut<'_, Result<T, TransportError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(TransportError::Connect("no-network".into())) })
    }
}

impl Transport for NoNetworkTransport {
    fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.fail()
    }

    fn get_with_timeout(
        &self,
        _url: &str,
        _timeout: std::time::Duration,
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.fail()
    }

    fn post_form(
        &self,
        _url: &str,
        _form: &[(String, String)],
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.fail()
    }

    fn post_json(&self, _url: &str, _body: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.fail()
    }

    fn get_with_auth(
        &self,
        _url: &str,
        _token: &str,
    ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
        self.fail()
    }

    fn post<'a>(&'a self, _req: HttpPost<'a>) -> BoxFut<'a, Result<HttpReplyEx, TransportError>> {
        self.fail()
    }
}
