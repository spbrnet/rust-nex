use bytemuck::{Pod, Zeroable};
use macros::RmcSerialize;
use rnex_core::rmc::structures::qbuffer::QBuffer;

#[derive(RmcSerialize, Debug)]
#[rmc_struct(0)]
struct UploadCompetitionData {
    winning_team: u32,
    splatfest_id: u32,
    unk_2: u32,
    unk_3: u32,
    team_id_1: u8,
    team_id_2: u8,
    unk_5: u32,
    player_data: QBuffer,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct UserData {
    name: [u16; 0x10],
}
