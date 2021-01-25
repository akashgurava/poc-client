use futures::stream::FuturesUnordered;
use futures::StreamExt;
// use futures::stream::FuturesUnordered;
// use futures::StreamExt;
use poc_client::client::Client;
use reqwest::{Method, Request, Url};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn single_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        // Mounting the mock on the mock server - it's now effective!
        .mount(&server)
        .await;

    let handler = Client::new();
    let url: Url = format!("{}/hello", &server.uri()).parse().unwrap();
    let request = Request::new(Method::GET, url);
    let response = handler.fetch(request).await;

    assert_eq!(response.unwrap().status().as_u16(), 200);
}

// #[tokio::test]
// async fn multi_sequence_request() {
//     let handler = Client::new();
//     let r1 = handler.fetch("Yippe".into()).await;
//     let r2 = handler.fetch("Yippe".into()).await;

//     assert_eq!(r1, "Success");
//     assert_eq!(r2, "Success");
// }

#[tokio::test]
async fn multi_parallel_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        // Mounting the mock on the mock server - it's now effective!
        .mount(&server)
        .await;

    let handler = Client::new();
    let url: Url = format!("{}/hello", &server.uri()).parse().unwrap();
    let request = Request::new(Method::GET, url);

    let mut x = (1..=3)
        .map(|_| handler.fetch(request.try_clone().unwrap()))
        .collect::<FuturesUnordered<_>>();

    let mut z = Vec::new();
    while let Some(y) = x.next().await {
        z.push(y.unwrap())
    }
    // let r1 = ;
    // let r2 = handler.fetch("Yippe".into());

    // let (r1, r2) = tokio::join!(r1, r2);

    assert_eq!(z[0].status().as_u16(), 200);
    assert_eq!(z[1].status().as_u16(), 200);
    assert_eq!(z[2].status().as_u16(), 200);
}
