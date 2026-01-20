use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "prudpv0")]{
        pub use prudpv0::*;
    } else if #[cfg(feature = "prudpv1")] {
        pub use prudpv1::*;
    } else {
        compile_error!("no proxy type has been set");
    }
}
