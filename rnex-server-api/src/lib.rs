pub mod meta {
    tonic::include_proto!("meta");
}
pub mod gatherings {
    tonic::include_proto!("gatherings");
}

pub mod auth {
    tonic::include_proto!("auth");
}
#[cfg(feature = "splatoon1")]
pub mod splatfest_results {
    tonic::include_proto!("splatfest_results");
}
