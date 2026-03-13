use serde::Serialize;

/// `/api/health` 响应（基础设施层 DTO，不属于领域）
#[derive(Serialize)]
pub struct HealthDto {
    pub status:      &'static str,
    pub cached_keys: usize,
}
