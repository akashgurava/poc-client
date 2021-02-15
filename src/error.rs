use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    // #[error("Unable to unwrap Client into inner Service. As there might be a reference which is not dropped.")]
    // ClientUnwrapError,
    #[error("Error while building http::Request. Reason: `{0}`.")]
    RequestBuildError(String),
    #[error("Error while sending request. Reason: `{0}`.")]
    RequestSendError(String),
    #[error("Error while parsing response to required type. Reason: `{0}`.")]
    ResponseParseError(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
