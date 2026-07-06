use macros::RmcSerialize;

#[derive(RmcSerialize, Debug, Clone, Copy, Default)]
#[rmc_struct(0)]
pub struct Data {}
