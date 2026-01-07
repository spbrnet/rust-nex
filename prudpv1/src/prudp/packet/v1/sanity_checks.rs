//! # PRUDPV1 Feature sanity checks
//!
//! Checks wether the set features are actually sensical or wether they are
//! nonsense and throws a compiler error incase of nonsense

#[cfg(feature = "friends")]
compile_error!(
    "friends uses prudpv0 instead of prudpv1, please do not enable both at the same time"
);

#[cfg(feature = "prudpv0")]
compile_error!("you cannot enable two prudp versions at the same time");
