use std::future::Future;
use std::time::Duration;
use tracing::error;

/// 通用异步重试函数
///
/// # 参数
/// * `f` - 需要重试的异步函数
/// * `retries` - 最大重试次数
/// * `delay` - 每次重试之间的延迟时间
/// * `operation_name` - 操作名称，用于日志记录
pub async fn retry<T, E, F, Fut>(
    mut f: F,
    retries: u32,
    delay: Duration,
    operation_name: &str,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug + Clone,
{
    // 如果重试次数为0，直接执行一次并返回结果
    if retries == 0 {
        return f().await;
    }

    let mut last_err = None;

    for attempt in 1..=retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let err_clone = e.clone();
                last_err = Some(e);
                error!(
                    target: "retry",
                    "{}失败，尝试重试 ({}/{}): {:?}",
                    operation_name, attempt, retries, err_clone
                );

                // 如果不是最后一次尝试，则等待
                if attempt < retries {
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_err.unwrap())
}
