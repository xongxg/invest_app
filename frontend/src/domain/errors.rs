use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum DomainError {
    #[error("网络请求失败: {0}")]
    NetworkError(String),

    #[error("API 响应解析失败: {0}")]
    ParseError(String),

    #[error("API 返回错误: {0}")]
    ApiError(String),

    #[error("未找到股票: {0}")]
    NotFound(String),

    #[error("CORS 限制：请通过后端代理访问 {0}")]
    CorsError(String),
}
