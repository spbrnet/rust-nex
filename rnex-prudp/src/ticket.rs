use std::io::Cursor;

use hmac::{Hmac, Mac};
use rc4::{KeyInit, Rc4, StreamCipher};
use rnex_util::byte::{IS_BIG_ENDIAN, ReadExtensions};
use rnex_util::{PID, account::Account};
use tracing::error;

use crate::{
    kerberos::{SESSION_KEY_LENGTH, TicketInternalData},
    read_buffer,
};

pub fn read_secure_connection_data(
    data: &[u8],
    act: &Account,
) -> Option<([u8; SESSION_KEY_LENGTH], PID, u32)> {
    let (mut ticket_data, remainder) = read_buffer(data)?;
    let (mut request_data, trailing) = read_buffer(remainder)?;
    if !trailing.is_empty() || ticket_data.len() != std::mem::size_of::<TicketInternalData>() + 16 {
        return None;
    }

    let server_key = act.nex_key;
    let ticket_data = authenticated_ciphertext(&mut ticket_data, &server_key)?;

    let mut rc4 = Rc4::new_from_slice(&server_key).expect("unable to init rc4 keystream");

    rc4.apply_keystream(ticket_data);

    if ticket_data.len() < std::mem::size_of::<TicketInternalData>() {
        error!("ticket_data buffer too small for TicketInternalData");
        return None;
    }

    let ticket_data_struct: TicketInternalData =
        bytemuck::pod_read_unaligned(&ticket_data[..std::mem::size_of::<TicketInternalData>()]);

    let TicketInternalData {
        session_key,
        pid: ticket_source_pid,
        issued_time,
    } = ticket_data_struct;

    let age = rnex_util::date_time::DateTime::now()
        .to_regular_time()
        .signed_duration_since(issued_time.to_regular_time())
        .num_seconds();
    if !(-60..=86400).contains(&age) {
        return None;
    }
    if request_data.len() != std::mem::size_of::<PID>() + 8 + 16 {
        return None;
    }
    let request_data = authenticated_ciphertext(&mut request_data, &session_key)?;

    let mut rc4: Rc4 = Rc4::new_from_slice(&session_key).expect("unable to init rc4 keystream");

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

fn authenticated_ciphertext<'a>(data: &'a mut [u8], key: &[u8]) -> Option<&'a mut [u8]> {
    let split = data.len().checked_sub(16)?;
    let (ciphertext, tag) = data.split_at_mut(split);
    let mut mac = <Hmac<md5::Md5> as KeyInit>::new_from_slice(key).ok()?;
    mac.update(ciphertext);
    mac.verify_slice(tag).ok()?;
    Some(ciphertext)
}
