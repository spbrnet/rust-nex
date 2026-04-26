use std::io::Cursor;

use log::{error, info};
use rc4::{KeyInit, Rc4, Rc4Core, StreamCipher, cipher::StreamCipherCoreWrapper};
use typenum::U16;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

use crate::{
    kerberos::{SESSION_KEY_LENGTH, SessionLengthTy, TicketInternalData, derive_key},
    nex::account::Account,
    rmc::structures::RmcSerialize,
};
use rnex_core::PID;

pub fn read_secure_connection_data(
    data: &[u8],
    act: &Account,
) -> Option<([u8; SESSION_KEY_LENGTH], PID, u32)> {
    let mut cursor = Cursor::new(data);

    let mut ticket_data: Vec<u8> = Vec::deserialize(&mut cursor).ok()?;
    let mut request_data: Vec<u8> = Vec::deserialize(&mut cursor).ok()?;
    info!("done request data {}", SESSION_KEY_LENGTH);

    let ticket_data_size = ticket_data.len();

    let ticket_data = &mut ticket_data[0..ticket_data_size - 0x10];

    let server_key = derive_key(act.pid, &act.kerbros_password[..]);

    let mut rc4: StreamCipherCoreWrapper<Rc4Core<U16>> =
        Rc4::new_from_slice(&server_key).expect("unable to init rc4 keystream");

    rc4.apply_keystream(ticket_data);

    let ticket_data: &TicketInternalData = match bytemuck::try_from_bytes(ticket_data) {
        Ok(v) => v,
        Err(e) => {
            error!("unable to read internal ticket data: {}", e);
            return None;
        }
    };

    // todo: add ticket expiration

    let TicketInternalData {
        session_key,
        pid: ticket_source_pid,
        issued_time,
    } = *ticket_data;

    // todo: add checking if tickets are signed with a valid md5-hmac
    let request_data_length = request_data.len();
    let request_data = &mut request_data[0..request_data_length - 0x10];

    let mut rc4: StreamCipherCoreWrapper<Rc4Core<SessionLengthTy>> =
        Rc4::new_from_slice(&session_key).expect("unable to init rc4 keystream");

    rc4.apply_keystream(request_data);

    let mut reqest_data_cursor = Cursor::new(request_data);

    let pid: PID = reqest_data_cursor.read_struct(IS_BIG_ENDIAN).ok()?;

    if pid != ticket_source_pid {
        let ticket_created_on = issued_time.to_regular_time();

        error!(
            "someone tried to spoof their pid, ticket was created on: {}",
            ticket_created_on.to_rfc2822()
        );
        return None;
    }

    let _cid: u32 = reqest_data_cursor.read_struct(IS_BIG_ENDIAN).ok()?;
    let response_check: u32 = reqest_data_cursor.read_struct(IS_BIG_ENDIAN).ok()?;

    Some((session_key, pid, response_check))
}
