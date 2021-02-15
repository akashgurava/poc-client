use http::{Method, Request};
use isahc::{AsyncBody, AsyncReadResponseExt};

use crate::client::Client;
use crate::error::{ClientError, Result};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    ticker_id: usize,
    #[serde(rename = "timeZone")]
    timezone: String,
    pre_close: String,
}

impl Client {
    pub async fn chart_quote(&self, ticker_ids: &str, period: &str) -> Result<Vec<Quote>> {
        let uri = format!(
            "https://quotes-gw.webullfintech.com/api/quote/charts/query?tickerIds={}&period={}",
            ticker_ids, period
        );
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(AsyncBody::empty())
            .map_err(|err| ClientError::RequestSendError(err.to_string()))?;
        let mut response = self.request(request).await?;
        let data = response
            .json::<Vec<Quote>>()
            .await
            .map_err(|err| ClientError::ResponseParseError(err.to_string()));
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quote_chart() {
        let client = Client::default();

        let data = client.chart_quote("913354090", "d1").await.unwrap();

        // We should get 1 ticker data
        assert_eq!(data.len(), 1);
    }
}
