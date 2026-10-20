use hmac::digest::KeyInit as HmacKeyInit;
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use rc4::{Rc4, StreamCipher};
use rnex_util::date_time::DateTime;
use subtle::ConstantTimeEq;

type HmacMd5 = Hmac<Md5>;
const CONNECTION_KEY: [u8; 15] = [
    0x26, 0xc3, 0x1f, 0x38, 0x1e, 0x46, 0xd6, 0xeb, 0x38, 0xe1, 0xaf, 0x6a, 0xb7, 0x0d, 0x11,
];

fn digest(data: &[u8]) -> [u8; 16] {
    Md5::digest(data).into()
}

fn mac(key: &[u8], data: &[u8]) -> [u8; 16] {
    let mut hmac = <HmacMd5 as HmacKeyInit>::new_from_slice(key).unwrap();
    hmac.update(data);
    hmac.finalize().into_bytes().into()
}

pub fn derive_key(password: &str, pid: u64) -> [u8; 16] {
    let initial = digest(password.as_bytes());
    let mut input = [0u8; 24];
    input[..16].copy_from_slice(&initial);
    input[16..].copy_from_slice(&pid.to_le_bytes());
    digest(&input)
}

pub fn connection_signature(address: std::net::SocketAddr) -> [u8; 16] {
    if let std::net::SocketAddr::V4(address) = address {
        let mut bytes = [0u8; 6];
        bytes[..4].copy_from_slice(&address.ip().octets());
        bytes[4..].copy_from_slice(&address.port().to_be_bytes());
        mac(&CONNECTION_KEY, &bytes)
    } else {
        rand::random()
    }
}

pub fn connect_signature(access_key: &str, connection_signature: &[u8]) -> [u8; 16] {
    let key = digest(access_key.as_bytes());
    let mut data = Vec::with_capacity(16 + connection_signature.len());
    data.extend_from_slice(&key);
    data.extend_from_slice(connection_signature);
    mac(&key, &data)
}

fn decrypt(key: &[u8], encrypted: &[u8]) -> Option<Vec<u8>> {
    let size = encrypted.len().checked_sub(16)?;
    let (ciphertext, signature) = encrypted.split_at(size);
    if mac(key, ciphertext).ct_eq(signature).unwrap_u8() != 1 {
        return None;
    }
    let mut plaintext = ciphertext.to_vec();
    let mut cipher = Rc4::new_from_slice(key).ok()?;
    cipher.apply_keystream(&mut plaintext);
    Some(plaintext)
}

pub fn encrypt(key: &[u8], plaintext: &[u8]) -> Option<Vec<u8>> {
    let mut result = plaintext.to_vec();
    let mut cipher = Rc4::new_from_slice(key).ok()?;
    cipher.apply_keystream(&mut result);
    result.extend_from_slice(&mac(key, &result));
    Some(result)
}

fn buffer(data: &[u8]) -> Option<Vec<u8>> {
    let len = u32::try_from(data.len()).ok()?;
    let mut result = Vec::with_capacity(4 + data.len());
    result.extend_from_slice(&len.to_le_bytes());
    result.extend_from_slice(data);
    Some(result)
}

fn take_buffer(data: &mut &[u8]) -> Option<Vec<u8>> {
    let len = u32::from_le_bytes(data.get(..4)?.try_into().ok()?) as usize;
    let result = data.get(4..4 + len)?.to_vec();
    *data = data.get(4 + len..)?;
    Some(result)
}

pub fn issue_ticket(
    source_pid: u64,
    target_pid: u64,
    source_key: &[u8],
    target_key: &[u8],
) -> Option<Vec<u8>> {
    let session_key: [u8; 32] = rand::random();
    let ticket_key: [u8; 16] = rand::random();
    let mut server_data = Vec::with_capacity(48);
    server_data.extend_from_slice(&DateTime::now().0.to_le_bytes());
    server_data.extend_from_slice(&source_pid.to_le_bytes());
    server_data.extend_from_slice(&session_key);
    let mut derived_input = Vec::with_capacity(target_key.len() + ticket_key.len());
    derived_input.extend_from_slice(target_key);
    derived_input.extend_from_slice(&ticket_key);
    let server_key = digest(&derived_input);
    let encrypted_server = encrypt(&server_key, &server_data)?;
    let mut internal = buffer(&ticket_key)?;
    internal.extend_from_slice(&buffer(&encrypted_server)?);
    let mut client_data = Vec::with_capacity(44 + internal.len());
    client_data.extend_from_slice(&session_key);
    client_data.extend_from_slice(&target_pid.to_le_bytes());
    client_data.extend_from_slice(&buffer(&internal)?);
    encrypt(source_key, &client_data)
}

pub fn verify_login(payload: &[u8], server_key: &[u8]) -> Option<(u64, Vec<u8>)> {
    let mut payload = payload;
    let internal = take_buffer(&mut payload)?;
    let request = take_buffer(&mut payload)?;
    if !payload.is_empty() {
        return None;
    }
    let mut internal = internal.as_slice();
    let ticket_key = take_buffer(&mut internal)?;
    let encrypted_ticket = take_buffer(&mut internal)?;
    if !internal.is_empty() || ticket_key.len() != 16 {
        return None;
    }
    let mut key_input = Vec::with_capacity(server_key.len() + ticket_key.len());
    key_input.extend_from_slice(server_key);
    key_input.extend_from_slice(&ticket_key);
    let ticket = decrypt(&digest(&key_input), &encrypted_ticket)?;
    if ticket.len() != 48 {
        return None;
    }
    let issued = DateTime(u64::from_le_bytes(ticket[..8].try_into().ok()?));
    let age = DateTime::now()
        .to_regular_time()
        .signed_duration_since(issued.to_regular_time())
        .num_seconds();
    if !(-60..=86400).contains(&age) {
        return None;
    }
    let source_pid = u64::from_le_bytes(ticket[8..16].try_into().ok()?);
    let request = decrypt(&ticket[16..], &request)?;
    if request.len() != 16 || u64::from_le_bytes(request[..8].try_into().ok()?) != source_pid {
        return None;
    }
    let check = u32::from_le_bytes(request[12..16].try_into().ok()?);
    let mut answer = Vec::with_capacity(8);
    answer.extend_from_slice(&4u32.to_le_bytes());
    answer.extend_from_slice(&check.wrapping_add(1).to_le_bytes());
    Some((source_pid, answer))
}