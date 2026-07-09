use std::borrow::Cow;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn rnex_release() -> String {
    let edition_piece = if let Some(e) = option_env!("EDITION") {
        format!("{}", e)
    } else {
        env!("FEATURESET").into()
    };

    format!(
        "rnex {} v{}({})",
        edition_piece,
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    )
}

pub async fn with_setup(f: impl AsyncFnOnce()) {
    println!("setting up logger and dotenv");
    dotenv::dotenv().ok();
    let _maybe_sentry = if let Ok(sentry_url) = std::env::var("SENTRY_URL") {
        Some(sentry::init((
            sentry_url,
            sentry::ClientOptions {
                release: Some(Cow::Owned(rnex_release())),
                send_default_pii: true,
                ..Default::default()
            },
        )))
    } else {
        None
    };
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry::integrations::tracing::layer())
        .try_init()
        .expect("failed to init tracing subscriber");

    f().await;

    /*ctrlc::set_handler(||{
        FORCE_EXIT.call_once_force(|_|{
            println!("attempting exit");
        });
    }).unwrap();*/
}
