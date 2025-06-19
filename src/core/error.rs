use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Service discovery error: {0}")]
    Discovery(String),
    
    #[error("File error: {0}")]
    File(String),
    
    #[error("Server error: {0}")]
    Server(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type AppResult<T> = Result<T, AppError>;
