pub static REQUEST_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(32);

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into()
}
