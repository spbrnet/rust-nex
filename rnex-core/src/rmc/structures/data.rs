use macros::RmcSerialize;

#[derive(RmcSerialize, Debug, Clone, Copy)]
#[rmc_struct(0)]
pub struct Data {}
