use bytemuck::{Pod, Zeroable};
use macros::RmcSerialize;
use rnex_core::rmc::structures::qbuffer::QBuffer;

#[derive(RmcSerialize, Debug)]
#[rmc_struct(0)]
pub struct UploadCompetitionData{
    pub unk_1/*?*/: u32,
    pub splatfest_id: u32,
    pub unk_2/*?*/: u32,
    pub score: u32,
    pub team_id: u8,
    pub team_win: u8,
    pub is_first_upload: bool,
    pub appdata: QBuffer,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct UserData {
    name: [u16; 0x10],
}
