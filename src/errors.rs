/// Tipos de erro centralizados da API.
///
/// `ApiError` é retornado por todos os handlers. Em `lib.rs` é convertido
/// em `Response` antes de chegar ao runtime do Spin, então o componente
/// nunca propaga um erro não-tratado para fora.
///
/// # Por que `thiserror` aqui e `anyhow` nos handlers?
/// - `thiserror`: camada de biblioteca — erros tipados, matcháveis, testáveis.
/// - `anyhow`: erros de infra dentro dos handlers (IO, parse) que chegam até
///   aqui via `#[from] anyhow::Error` e se tornam `ApiError::Internal`.

use spin_sdk::http::Response;

use crate::helpers::error_json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Parâmetro ausente ou com valor inválido (HTTP 400).
    #[error("{0}")]
    BadRequest(String),

    /// Coordenada fora da área operacional (HTTP 422).
    #[error("{0}")]
    UnprocessableEntity(String),

    /// Recurso não encontrado (HTTP 404).
    #[error("{0}")]
    NotFound(String),

    /// Upstream (Nominatim ou OSRM) retornou erro (HTTP 502).
    #[error("upstream {service} falhou: {message}")]
    BadGateway {
        service: &'static str,
        message: String,
    },

    /// Erro interno inesperado — wraps anyhow (HTTP 500).
    #[error("erro interno: {0}")]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    pub fn status(&self) -> u16 {
        match self {
            Self::BadRequest(_) => 400,
            Self::NotFound(_) => 404,
            Self::UnprocessableEntity(_) => 422,
            Self::BadGateway { .. } => 502,
            Self::Internal(_) => 500,
        }
    }

    /// Erros de servidor merecem log; erros de cliente (4xx) não.
    pub fn is_server_error(&self) -> bool {
        self.status() >= 500
    }

    pub fn bad_gateway(service: &'static str, msg: impl Into<String>) -> Self {
        Self::BadGateway {
            service,
            message: msg.into(),
        }
    }
}

/// Converte `ApiError` → `Response` diretamente.
/// Permite usar `.map_err(Response::from)` em qualquer handler.
impl From<ApiError> for Response {
    fn from(e: ApiError) -> Response {
        error_json(e.status(), &e.to_string())
    }
}

// ── Helpers de construção ─────────────────────────────────────────────────

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn unprocessable(msg: impl Into<String>) -> Self {
        Self::UnprocessableEntity(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
}