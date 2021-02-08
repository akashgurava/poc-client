// #![allow(dead_code, unused_imports, unused_variables)]

use std::collections::HashMap;
use std::str::FromStr;

use futures::future::join_all;
use http::{Method, Request, Uri};
use isahc::prelude::AsyncReadResponseExt;
use isahc::HttpClient as Client;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use tower::{service_fn, Service, ServiceBuilder, ServiceExt};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Quote {
    ticker_id: usize,
    #[serde(rename = "timeZone")]
    timezone: String,
    pre_close: String,
}

fn mean(numbers: &[u128]) -> f32 {
    numbers.iter().sum::<u128>() as f32 / numbers.len() as f32
}

fn median(numbers: &mut [u128]) -> u128 {
    numbers.sort();
    let mid = numbers.len() / 2;
    numbers[mid]
}

fn mode(numbers: &[u128]) -> u128 {
    let mut occurrences = HashMap::new();

    for &value in numbers {
        *occurrences.entry(value).or_insert(0) += 1;
    }

    occurrences
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(val, _)| val)
        .expect("Cannot compute the mode of zero numbers")
}

pub async fn no_limits_isahc(count: usize) {
    let start = Instant::now();
    let (stx, mut srx) = mpsc::unbounded_channel::<u8>();
    let (ftx, mut frx) = mpsc::unbounded_channel::<u8>();
    let (dtx, mut drx) = mpsc::unbounded_channel::<u128>();

    let url =
        "https://quotes-gw.webullfintech.com/api/quote/charts/query?tickerIds=913354090&period=d1"
            .to_string();
    let requests = (1..=count)
        .map(|_| {
            let url = url.clone();
            let s_send = stx.clone();
            let f_send = ftx.clone();
            let d_send = dtx.clone();
            tokio::spawn(async move {
                let t_start = Instant::now();
                let response = isahc::get_async(&url).await;
                match response {
                    Ok(mut response) => {
                        if response.status().as_u16() == 200 {
                            let data = response.json::<Vec<Quote>>().await;
                            match data {
                                Ok(_) => {
                                    s_send.send(1).unwrap();
                                }
                                Err(_) => {
                                    f_send.send(1).unwrap();
                                }
                            }
                        } else {
                            f_send.send(1).unwrap();
                        }
                    }
                    Err(_) => {
                        f_send.send(1).unwrap();
                    }
                }
                d_send.send(t_start.elapsed().as_millis()).unwrap();
            })
        })
        .collect::<Vec<_>>();

    drop(stx);
    drop(ftx);
    drop(dtx);

    let mut req_dur = Vec::new();
    let mut success = 0usize;
    let mut fail = 0usize;
    while let Some(_) = srx.recv().await {
        success += 1;
    }
    while let Some(_) = frx.recv().await {
        fail += 1;
    }
    while let Some(dur) = drx.recv().await {
        req_dur.push(dur);
    }
    join_all(requests).await;

    println!("Total sent requests: {}", count);
    println!(
        "Success: {}({}%)",
        success,
        ((success / count) * 100) as usize
    );
    println!("Fail: {}({}%)", fail, ((fail / count) * 100) as usize);
    println!(
        "No Limits: Total - {}ms > Mean - {}ms > Median - {}ms > Mode - {}ms > Min - {}ms > Max - {}ms",
        start.elapsed().as_millis(),
        mean(&req_dur),
        median(&mut req_dur.clone()),
        mode(&req_dur),
        req_dur.iter().min().unwrap(),
        req_dur.iter().max().unwrap(),
    );
}

pub async fn with_service_isahc(count: usize) {
    let start = Instant::now();
    let url =
        "https://quotes-gw.webullfintech.com/api/quote/charts/query?tickerIds=913354090&period=d1";

    // Create Service
    let client = Client::new();
    let service = service_fn(move |request: Request<()>| {
        let client = client.clone().unwrap();

        async move {
            let response = client.send_async(request).await;
            response
        }
    });
    let mut service = ServiceBuilder::new()
        .rate_limit(1, Duration::from_millis(5))
        // .rate_limit(1, Duration::from_micros(200))
        // .rate_limit(100, Duration::from_secs(1))
        // .rate_limit(10, Duration::from_secs(4))
        .concurrency_limit(50)
        .service(service);

    // Prepare channels
    let (stx, mut srx) = mpsc::unbounded_channel::<u8>();
    let (ftx, mut frx) = mpsc::unbounded_channel::<u8>();
    let (dtx, mut drx) = mpsc::unbounded_channel::<u128>();

    let mut req_num = 1;
    loop {
        if count < req_num {
            break;
        }

        let s_send = stx.clone();
        let f_send = ftx.clone();
        let d_send = dtx.clone();

        let request = Request::builder()
            .method(Method::GET)
            .uri(Uri::from_str(url).unwrap())
            .body(())
            .unwrap();
        let fut = service.ready_and().await.unwrap().call(request);

        tokio::spawn(async move {
            let t_start = Instant::now();
            let response = fut.await;
            match response {
                Ok(mut response) => {
                    if response.status().as_u16() == 200 {
                        let data = response.json::<Vec<Quote>>().await;
                        match data {
                            Ok(_) => {
                                s_send.send(1).unwrap();
                            }
                            Err(_) => {
                                f_send.send(1).unwrap();
                            }
                        }
                    } else {
                        f_send.send(1).unwrap();
                    }
                }
                Err(_) => {
                    f_send.send(1).unwrap();
                }
            }
            d_send.send(t_start.elapsed().as_millis()).unwrap();
        });
        req_num += 1;
    }

    drop(stx);
    drop(ftx);
    drop(dtx);

    let mut req_dur = Vec::new();
    let mut success = 0usize;
    let mut fail = 0usize;
    while let Some(_) = srx.recv().await {
        success += 1;
    }
    while let Some(_) = frx.recv().await {
        fail += 1;
    }
    while let Some(dur) = drx.recv().await {
        req_dur.push(dur);
    }

    println!("Total sent requests: {}", count);
    println!(
        "Success: {}({}%)",
        success,
        ((success / count) * 100) as usize
    );
    println!("Fail: {}({}%)", fail, ((fail / count) * 100) as usize);
    println!(
        "With Service: Total - {}ms > Mean - {}ms > Median - {}ms > Mode - {}ms > Min - {}ms > Max - {}ms",
        start.elapsed().as_millis(),
        mean(&req_dur),
        median(&mut req_dur.clone()),
        mode(&req_dur),
        req_dur.iter().min().unwrap(),
        req_dur.iter().max().unwrap(),
    );
}
