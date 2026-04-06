use std::io;
use std::io::{Read, Seek, Write};
use bytemuck::bytes_of;
use log::error;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};
use crate::rmc::response::{ErrorCode, RMCResponseResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RMCMessage{
    pub protocol_id: u16,
    pub call_id: u32,
    pub method_id: u32,

    pub rest_of_data: Vec<u8>
}

impl RMCMessage{
    pub fn new(stream: &mut (impl Seek + Read)) -> io::Result<Self>{
        let size: u32 = stream.read_struct(IS_BIG_ENDIAN)?;

        let mut header_size = 1 + 4 + 4;

        let protocol_id: u8 = stream.read_struct(IS_BIG_ENDIAN)?;
        let protocol_id= protocol_id & (!0x80);

        let protocol_id: u16 = match protocol_id{
            0x7F => {
                header_size += 2;
                stream.read_struct(IS_BIG_ENDIAN)?
            },
            _ => protocol_id as u16
        };

        let call_id = stream.read_struct(IS_BIG_ENDIAN)?;
        let method_id = stream.read_struct(IS_BIG_ENDIAN)?;

        let mut rest_of_data = Vec::new();

        stream.read_to_end(&mut rest_of_data)?;

        if header_size + rest_of_data.len() != size as usize {
            error!("received incorrect rmc packet: expected size {} but found {}", size, header_size + rest_of_data.len());
        }

        // println!("rmc packet: protoid: {}, method id: {}", protocol_id, method_id);
        // println!("{}", hex::encode(&rest_of_data));

        //stream.
        Ok(Self{
            protocol_id,
            method_id,
            call_id,
            rest_of_data
        })
    }

    pub fn to_data(&self) -> Vec<u8>{
        let size = (1 + 4 + 4 + self.rest_of_data.len()) as u32;

        let mut output = Vec::new();

        output.write_all(bytes_of(&size)).expect("unable to write size");

        let proto_id = self.protocol_id as u8 | 0x80;

        output.write_all(bytes_of(&proto_id)).expect("unable to write size");

        output.write_all(bytes_of(&self.call_id)).expect("unable to write size");
        output.write_all(bytes_of(&self.method_id)).expect("unable to write size");

        output.write_all(&self.rest_of_data).expect("unable to write data");

        output
    }

    pub fn error_result_with_code(&self, error_code: ErrorCode) -> RMCResponseResult{
        RMCResponseResult::Error {
            call_id: self.call_id,
            error_code
        }
    }

    pub fn success_with_data(&self, data: Vec<u8>) -> RMCResponseResult{
        RMCResponseResult::Success {
            call_id: self.call_id,
            method_id: self.method_id,
            data
        }
    }
}