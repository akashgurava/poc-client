use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Unable to unwrap Client into inner Service. As there might be a reference which is not dropped.")]
    ClientUnwrapError,
    #[error("Unable to convert from Request to http::Request. Reason: `{0}`.")]
    RequestConversionError(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
