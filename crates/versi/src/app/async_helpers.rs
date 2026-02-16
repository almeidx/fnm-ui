use std::future::Future;
use std::time::Duration;

use log::debug;

use crate::error::AppError;

pub(super) async fn run_with_timeout<T, E, F, M>(
    timeout: Duration,
    timeout_operation: &'static str,
    future: F,
    map_error: M,
) -> Result<T, AppError>
where
    F: Future<Output = Result<T, E>>,
    M: FnOnce(E) -> AppError,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(map_error(error)),
        Err(_) => Err(AppError::timeout(timeout_operation, timeout.as_secs())),
    }
}

pub(super) async fn retry_with_delays<T, Op, Fut>(
    operation_name: &'static str,
    retry_delays_secs: &[u64],
    mut operation: Op,
) -> Result<T, AppError>
where
    Op: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut last_err = AppError::message("Unknown error");

    for (attempt, &delay_secs) in retry_delays_secs.iter().enumerate() {
        if delay_secs > 0 {
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        }

        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                debug!(
                    "{} attempt {} failed: {}",
                    operation_name,
                    attempt + 1,
                    error
                );
                last_err = error;
            }
        }
    }

    Err(last_err)
}
