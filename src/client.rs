use std::convert::TryInto;
use std::str::from_utf8;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use http::{Request as HttpRequest, Version};
use hyper::body::to_bytes;
use hyper::client::HttpConnector;
use hyper::{Body, Client as HyClient};
use hyper_tls::HttpsConnector;
use log::{error, info, warn};
use serde::{de::DeserializeOwned, Serialize};
use tokio::time::{sleep, Duration, Instant};
use tokio_stream as stream;
use typed_builder::TypedBuilder;

use crate::request::{self, HyperRequest, Method, Request};

type HyperClient = HyClient<HttpsConnector<HttpConnector>>;

#[derive(Debug, TypedBuilder)]
pub struct Client {
    #[builder(default_code = "HyClient::builder().build::<_, hyper::Body>(HttpsConnector::new())")]
    client: HyperClient,
    #[builder(default_code = "5")]
    max_tries: u8,
    #[builder(default_code = "Duration::from_millis(600)")]
    throttle: Duration,
    #[builder(default_code = "5")]
    concurrency: usize,
    #[builder(default_code = "vec![
        Duration::from_millis(1200),
        Duration::from_millis(1800),
        Duration::from_millis(2400),
        Duration::from_millis(3000),
    ]")]
    backoff: Vec<Duration>,
    #[builder(default, setter(strip_option))]
    version: Option<Version>,
}

impl Default for Client {
    fn default() -> Self {
        Client::builder().build()
    }
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    async fn try_fetch<T>(&self, request: Request) -> Result<T>
    where
        T: DeserializeOwned,
    {
        // let body = body.unwrap_or_default();
        let http_req: HyperRequest = request.try_into()?;
        let response = self.client.request(http_req).await?;
        let status = response.status();
        let body = to_bytes(response)
            .await
            .context("Parsing response failed")?;
        let data = if status.is_success() {
            serde_json::from_slice::<T>(&body).context("Deserialize to required struct failed")?
        } else {
            let error = from_utf8(&body).context("Unable to parse response body as string")?;
            return Err(anyhow!("Error response - {}", error));
        };
        Ok(data)
    }

    pub async fn fetch<T>(&self, request: Request) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let start = Instant::now();

        for attempt in 1..=self.max_tries {
            let response = self.try_fetch::<T>(request.clone()).await;
            match response {
                Ok(data) => {
                    info!(
                        "{}th attempt > Success > Time elapsed {} secs",
                        attempt,
                        start.elapsed().as_secs()
                    );
                    return Some(data);
                }
                Err(err) => {
                    if attempt < self.max_tries {
                        warn!(
                            "{}th attempt failed > Reason - {}",
                            attempt,
                            err.to_string(),
                        );
                        let dur = self.backoff.get((attempt - 1) as usize).unwrap();
                        sleep(dur.clone()).await;
                    } else {
                        error!(
                            "{}th attempt failed > Reason - {}",
                            attempt,
                            err.to_string(),
                        );
                    }
                }
            }
        }
        None
    }

    async fn fetch_single<T>(&self, identifier: usize, request: Request) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let start = Instant::now();
        for attempt in 1..=self.max_tries {
            let response = self.try_fetch::<T>(request.clone()).await;
            match response {
                Ok(data) => {
                    info!(
                        "{}th request > {}th attempt > Success > Time elapsed {} secs",
                        identifier,
                        attempt,
                        start.elapsed().as_secs()
                    );
                    return Some(data);
                }
                Err(err) => {
                    if attempt < self.max_tries {
                        warn!(
                            "{}th request > {}th attempt failed > Reason - {}",
                            identifier,
                            attempt,
                            err.to_string(),
                        );
                        let dur = self.backoff.get((attempt - 1) as usize).unwrap();
                        sleep(dur.clone()).await;
                    } else {
                        error!(
                            "{}th request > {}th attempt failed > Reason - {}",
                            identifier,
                            attempt,
                            err.to_string(),
                        );
                    }
                }
            }
        }
        None
    }

    pub async fn fetch_multiple<Q, T>(&self, requests: Vec<Request>) -> Vec<Option<T>>
    where
        Q: Serialize,
        T: DeserializeOwned,
    {
        let requests = stream::iter(requests);
        let data = stream::StreamExt::throttle(requests, self.throttle)
            .enumerate()
            .map(|(mut identifier, request)| async move {
                identifier += 1;
                self.fetch_single::<T>(identifier, request).await
            })
            .buffered(self.concurrency)
            .collect::<Vec<_>>()
            .await;

        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct TestHW {
        msg: String,
    }

    #[test]
    fn test_client_builder() {
        Client::builder().build();
    }

    #[tokio::test]
    async fn simple_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/hello"))
            .respond_with(ResponseTemplate::new(200).set_body_json(TestHW {
                msg: "hello world".into(),
            }))
            // Mounting the mock on the mock server - it's now effective!
            .mount(&server)
            .await;
        let client = Client::new();
        let url = format!("{}/hello", &server.uri());
        let request = Request::new(Method::Get, url);

        let data = client.fetch::<TestHW>(request).await;

        assert_eq!(data.is_some(), true);
        assert_eq!(
            data.unwrap(),
            TestHW {
                msg: "hello world".into()
            }
        );
    }

    #[tokio::test]
    async fn simple_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/hello"))
            .respond_with(ResponseTemplate::new(500).set_body_json(TestHW {
                msg: "Error".into(),
            }))
            .mount(&server)
            .await;
        let client = Client::new();
        let url = format!("{}/hello", &server.uri());
        let request = Request::new(Method::Get, url);

        let data = client.fetch::<TestHW>(request).await;

        assert_eq!(data.is_none(), true);
    }
}
