use http::{Request, Response};
use isahc::{AsyncBody, HttpClient};

use crate::error::{ClientError, Result};

#[derive(Clone)]
pub struct Client {
    inner: HttpClient,
}

impl Default for Client {
    #[inline]
    fn default() -> Self {
        Client {
            inner: HttpClient::new().expect("Unable to create default empty client."),
        }
    }
}

impl Client {
    #[inline]
    pub fn new(client: HttpClient) -> Self {
        Client { inner: client }
    }

    #[inline]
    pub fn inner(self) -> HttpClient {
        self.inner
    }

    pub(crate) async fn request(&self, request: Request<AsyncBody>) -> Result<Response<AsyncBody>> {
        self.inner
            .send_async(request)
            .await
            .map_err(|err| ClientError::RequestSendError(err.to_string()))
    }
}
