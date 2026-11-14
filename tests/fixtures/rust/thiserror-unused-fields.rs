#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API request failed: status {status}")]
    Api {
        status: u16,
        message: Option<String>,
    },
}
