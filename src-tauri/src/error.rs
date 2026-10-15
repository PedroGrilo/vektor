use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Funcionalidade não suportada neste sistema")]
    Unsupported,
    #[error("Não disponível: {0}")]
    Unavailable(String),
    #[error("Valor inválido: {0}")]
    InvalidInput(String),
    #[error("{0}")]
    Msg(String),
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro de JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn msg(m: impl Into<String>) -> Self {
        Error::Msg(m.into())
    }
}

// Tauri commands send errors to the UI as plain strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
