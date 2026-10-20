use rnex_server::with_setup;

#[tokio::main]
async fn main() {
    with_setup(async || {
        proxy::start().await.expect("PRUDP Lite listener stopped");
        Ok(())
    })
    .await;
}
