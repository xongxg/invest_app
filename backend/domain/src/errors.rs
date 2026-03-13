use thiserror::Error;

/// 领域层错误（与技术细节无关）
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("外部 API 错误: {0}")]
    External(String),

    #[error("未找到: {0}")]
    NotFound(String),

    #[error("内部错误: {0}")]
    Internal(String),
}
