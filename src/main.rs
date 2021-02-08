use poc_client::hac::{no_limits_isahc, with_service_isahc};
use poc_client::reqw::{no_limits_req, with_service_req};

#[tokio::main]
async fn main() {
    let req_count = 500;

    no_limits_isahc(req_count).await;
    with_service_isahc(req_count).await;

    no_limits_req(req_count).await;
    with_service_req(req_count).await;
}
