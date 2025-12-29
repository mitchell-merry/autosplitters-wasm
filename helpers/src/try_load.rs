use core::time::Duration;
use std::error::Error;
use std::future::Future;

pub async fn wait_try_load<T, F, Fut>(load_fn: F) -> T
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Box<dyn Error>>>,
{
    wait_try_load_millis(load_fn, Duration::from_millis(100)).await
}

pub async fn wait_try_load_millis<T, F, Fut>(load_fn: F, cooldown: Duration) -> T
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Box<dyn Error>>>,
{
    asr::print_message("=> attempting try_load");

    let result = loop {
        let result = load_fn().await;

        let error = match result {
            Ok(result) => break result,
            Err(e) => e,
        };

        asr::print_message(&format!(
            "=> try_load unsuccessful, trying again in {}ms! with error: {}",
            cooldown.as_millis(),
            error
        ));
        asr::future::sleep(cooldown).await;
    };

    asr::print_message("=> try_load successful!");

    result
}
