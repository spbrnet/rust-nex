use macros::RmcSerialize;

#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(0)]
pub struct ResultsRange{
    pub offset: u32,
    pub size: u32
}