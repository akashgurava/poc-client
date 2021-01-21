use std::convert::TryFrom;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Error;
use erased_serde::Serialize;
use http::{Method as HttpMethod, Request as HttpRequest, Uri, Version};
use hyper::Body;
use typed_builder::TypedBuilder;

pub type HyperRequest = HttpRequest<Body>;

pub fn to_string<T>(input: T) -> Result<String, Error>
where
    T: serde::Serialize,
{
    serde_urlencoded::to_string(input).map_err(|error| error.into())
}

pub trait Query: Serialize + Debug {
    fn to_string(&self) -> Result<String, Error>;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Method {
    Get,
}

impl From<Method> for HttpMethod {
    fn from(method: Method) -> Self {
        match method {
            Method::Get => HttpMethod::GET,
        }
    }
}

impl Default for Method {
    fn default() -> Self {
        Method::Get
    }
}

#[derive(Debug, TypedBuilder, Clone)]
pub struct Request {
    method: Method,
    url: String,
    #[builder(default)]
    version: Option<Version>,
    #[builder(default, setter(strip_option))]
    query: Option<Arc<dyn Query>>,
}

impl Request {
    pub fn new(method: Method, url: String) -> Self {
        Request::builder().method(method).url(url).build()
    }

    pub fn new_all(
        method: Method,
        url: String,
        version: Option<Version>,
        query: Option<Arc<dyn Query>>,
    ) -> Self {
        Self {
            method,
            url,
            version,
            query,
        }
    }

    pub fn to_http_with_body(self, body: Body) -> Result<HyperRequest, Error> {
        let uri = match self.query {
            None => self.url.parse::<Uri>()?,
            Some(query) => {
                let query = query.to_string()?;
                format!("{}{}", self.url, query).parse::<Uri>()?
            }
        };

        let request = HttpRequest::builder().uri(uri).method(self.method);
        let request = match self.version {
            Some(version) => request.version(version),
            _ => request,
        };
        request.body(body).map_err(|error| error.into())
    }
}

impl TryFrom<Request> for HyperRequest {
    type Error = Error;
    fn try_from(request: Request) -> Result<Self, Self::Error> {
        request.to_http_with_body(Body::empty())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;

    #[derive(Serialize, Debug)]
    struct ReqQuery {
        start: u32,
        count: u32,
    }

    impl Query for ReqQuery {
        fn to_string(&self) -> Result<String, Error> {
            to_string(self)
        }
    }

    #[test]
    fn test_request_new() {
        Request::new(Method::Get, "jnjs".into());
        Request::new_all(Method::Get, "jnjs".into(), None, None);
    }

    #[test]
    fn test_request_builder() {
        // Method and Url
        Request::builder()
            .method(Method::Get)
            .url("hu".into())
            .build();
        // Method, Url and Version
        Request::builder()
            .method(Method::Get)
            .url("hu".into())
            .version(Some(Version::HTTP_2))
            .build();
        // Method, Url, Version and Query
        Request::builder()
            .method(Method::Get)
            .url("hu".into())
            .version(Some(Version::HTTP_2))
            .query(Arc::new(ReqQuery {
                start: 10,
                count: 10,
            }))
            .build();
    }

    #[test]
    fn test_request_to_http() {
        // Method and Url
        let request = Request::builder()
            .method(Method::Get)
            .url("hu".into())
            .build();
        let httpreq: Result<HttpRequest<Body>, _> =
            request.clone().to_http_with_body(Body::empty());
        let httpreq: Result<HttpRequest<Body>, _> = request.try_into();

        let request = Request::builder()
            .method(Method::Get)
            .url("hu".into())
            .version(Some(Version::HTTP_2))
            .query(Arc::new(ReqQuery {
                start: 10,
                count: 10,
            }))
            .build();
        let httpreq: Result<HttpRequest<Body>, _> =
            request.clone().to_http_with_body(Body::empty());
        let httpreq: Result<HttpRequest<Body>, _> = request.try_into();
    }
}
