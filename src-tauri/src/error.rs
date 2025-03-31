use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("io error: `{0}`")]
    Io(#[from] std::io::Error),

    #[error("generic anyhow error: `{0}`")]
    Anyhow(#[from] anyhow::Error),
}
