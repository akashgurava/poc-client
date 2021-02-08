use std::convert::{TryFrom, TryInto};
use std::sync::Arc;
use std::time::Duration;

use hyper::client::HttpConnector;
use hyper::http::method::Method;
use hyper::http::uri::Uri;
use hyper::service::Service;
use hyper_tls::HttpsConnector;
use tower::limit::*;
use tower::Layer;

use crate::error::{ClientError, Result};

type HyperRequest = hyper::Request<hyper::Body>;
type HyperClient = hyper::Client<HttpsConnector<HttpConnector>>;

pub struct Request {
    method: Method,
    uri: String,
}

impl TryFrom<Request> for HyperRequest {
    type Error = ClientError;

    fn try_from(request: Request) -> Result<Self> {
        let builder = hyper::Request::builder();

        let Request { method, uri } = request;

        let uri = Uri::try_from(uri)
            .map_err(|err| ClientError::RequestConversionError(err.to_string()))?;
        builder
            .method(method)
            .uri(uri)
            .body(hyper::Body::empty())
            .map_err(|err| ClientError::RequestConversionError(err.to_string()))
    }
}

pub struct Response {}

struct ClientRef<T>
where
    T: Service<HyperRequest>,
{
    inner: T,
}

#[derive(Clone)]
pub struct Client<T>
where
    T: Service<HyperRequest>,
{
    client: Arc<ClientRef<T>>,
}

impl From<HyperClient> for Client<HyperClient> {
    fn from(client: HyperClient) -> Self {
        Client {
            client: Arc::new(ClientRef { inner: client }),
        }
    }
}

/// A default [Client] is created with a [HyperClient].
///
/// If you you want create a Client with pre-defined [Service],
/// use [Client]::[new].
impl Default for Client<HyperClient> {
    fn default() -> Self {
        let connector = HttpsConnector::new();
        let client = hyper::Client::builder().build(connector);

        Client::from(client)
    }
}

impl<T> Client<T>
where
    T: Service<HyperRequest>,
{
    pub fn new(client: T) -> Self {
        Client {
            client: Arc::new(ClientRef { inner: client }),
        }
    }

    /// Try to fetch inner [HyperClient] or [Service]. If there are any active
    /// references to inner client an error is returned.
    pub fn inner(self) -> Result<T> {
        if let Ok(client_ref) = Arc::try_unwrap(self.client) {
            Ok(client_ref.inner)
        } else {
            Err(ClientError::ClientUnwrapError)
        }
    }

    /// Try to add a [RateLimitLayer] to [Client]. If there are any active
    /// references to inner client an error is returned.
    pub fn add_rate_limit(self, num: u64, per: Duration) -> Result<Client<RateLimit<T>>> {
        let layer = RateLimitLayer::new(num, per);
        let inner = self.inner()?;
        let client = layer.layer(inner);
        Ok(Client::new(client))
    }

    /// Try to add a [ConcurrencyLimitLayer] to [Client]. If there are any active
    /// references to inner client an error is returned.
    pub fn add_concurrency_limit(self, max: usize) -> Result<Client<ConcurrencyLimit<T>>> {
        let layer = ConcurrencyLimitLayer::new(max);
        let inner = self.inner()?;
        let client = layer.layer(inner);
        Ok(Client::new(client))
    }

    pub async fn request(&self, request: Request) -> Result<Response> {
        let request: HyperRequest = request.try_into()?;
        todo!()
    }

    // pub async fn request<R: TryInto<HyperRequest>>(&self, request: R) -> Result<Response> {
    //     let request: HyperRequest = match request.try_into() {
    //         Ok(request) => request,
    //         Err(error) => match error.kind() {},
    //     };
    //     todo!()
    // }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use hyper_tls::HttpsConnector;
    use tower::ServiceBuilder;

    use super::*;

    #[test]
    fn test_only_client() {
        let connector = HttpsConnector::new();
        let client = hyper::Client::builder().build(connector);

        Client::new(client);
    }

    #[test]
    fn test_client_with_layers() {
        let connector = HttpsConnector::new();
        let client = hyper::Client::builder().build(connector);

        let client = ServiceBuilder::new()
            .rate_limit(10, Duration::from_secs(1))
            .service(client);

        Client::new(client);
    }

    #[tokio::test]
    async fn test_client_add_layers() {
        let client = Client::default();

        // Test fail if dangling reference
        let clone = client.clone();
        let inner = client.inner();
        assert!(inner.is_err());
        // Since `client` is consumed in `inner` call. clone.inner() should succeed.
        let inner = clone.inner();
        assert!(inner.is_ok());

        let client = Client::default();
        // Add rate limit
        let rate_client = client.add_rate_limit(1, Duration::from_secs(3)).unwrap();
        // Add concurrency limit
        let _con_rate_client = rate_client.add_concurrency_limit(1).unwrap();
    }

    #[tokio::test]
    async fn test_client_request() {
        let client = Client::default();
        client
            .request(Request {
                method: Method::GET,
                uri: "https://httpbin.org/code/200".into(),
            })
            .await
            .unwrap();
    }
}
