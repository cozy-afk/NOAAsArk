use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("config error: {0}")]
    Config(String),
    #[error("invalid market: {0}")]
    InvalidMarket(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
