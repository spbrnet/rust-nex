use std::net::Ipv4Addr;
use log::error;
use std::fmt::{Debug, Display, Formatter, Write};
use std::io::Read;
use crate::prudp::station_url::Type::{PRUDP, PRUDPS, UDP};
use crate::prudp::station_url::UrlOptions::{Address, ConnectionID, NatFiltering, NatMapping, NatType, Platform, PMP, Port, PrincipalID, RVConnectionID, StreamID, StreamType, UPNP, PID};
use crate::rmc::structures::Error::StationUrlInvalid;
use crate::rmc::structures::RmcSerialize;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Type{
    UDP,
    PRUDP,
    PRUDPS
}

pub mod nat_types{
    pub const BEHIND_NAT: u8 = 1;
    pub const PUBLIC: u8 = 2;
}

#[derive(Clone, Eq, PartialEq)]
pub enum UrlOptions {
    Address(Ipv4Addr),
    Port(u16),
    StreamType(u8),
    StreamID(u8),
    ConnectionID(u8),
    PrincipalID(u32),
    NatType(u8),
    NatMapping(u8),
    NatFiltering(u8),
    UPNP(u8),
    RVConnectionID(u32),
    Platform(u8),
    PMP(u8),
    PID(u32),

}

#[derive(Clone, PartialEq, Eq)]
pub struct StationUrl{
    pub url_type: Type,
    pub options: Vec<UrlOptions>
}

impl StationUrl{
    pub fn read_options(options: &str) -> Option<Vec<UrlOptions>>{
        let mut options_out = Vec::new();

        for option in options.split(';'){
            if option == "" { continue; }
            let mut option_parts = option.split('=');
            let option_name= option_parts.next()?.to_ascii_lowercase();
            let option_value = option_parts.next()?;

            match option_name.as_ref(){
                "address" => {
                    options_out.push(Address(option_value.parse().ok()?))
                },
                "port" => {
                    options_out.push(Port(option_value.parse().ok()?))
                }
                "natf" => {
                    options_out.push(NatFiltering(option_value.parse().ok()?))
                }
                "natm" => {
                    options_out.push(NatMapping(option_value.parse().ok()?))
                }
                "sid" => {
                    options_out.push(StreamID(option_value.parse().ok()?))
                }
                "upnp" => {
                    options_out.push(UPNP(option_value.parse().ok()?))
                }
                "type" => {
                    options_out.push(NatType(option_value.parse().ok()?))
                }
                "stream" => {
                    options_out.push(StreamType(option_value.parse().ok()?))
                }
                "RVCID" => {
                    options_out.push(RVConnectionID(option_value.parse().ok()?))
                }
                "rvcid" => {
                    options_out.push(RVConnectionID(option_value.parse().ok()?))
                }
                "pl" => {
                    options_out.push(Platform(option_value.parse().ok()?))
                }
                "pmp" => {
                    options_out.push(PMP(option_value.parse().ok()?))
                },
                "pid" => {
                    options_out.push(PID(option_value.parse().ok()?))
                },
                "PID" => {
                    options_out.push(PID(option_value.parse().ok()?))
                },
                _ => {
                    error!("unimplemented option type, skipping: {}", option_name);
                }
            }



        }

        Some(options_out)
    }
}

impl TryFrom<&str> for StationUrl{
    type Error = ();

    fn try_from(value: &str) -> Result<Self, ()> {
        let (url_type, options) = value.split_at(value.find(":/").ok_or(())?);

        let options = &options[2..];

        let url_type = match url_type{
            "udp" => UDP,
            "prudp" => PRUDP,
            "prudps" => PRUDPS,
            _ => return Err(())
        };

        let options = Self::read_options(options).ok_or(())?;

        Ok(
            Self{
                url_type,
                options
            }
        )
    }
}

impl<'a> Into<String> for &'a StationUrl{
    fn into(self) -> String {
        let mut url = match self.url_type{
            UDP => "udp:/",
            PRUDP => "prudp:/",
            PRUDPS => "prudps:/"
        }.to_owned();

        for option in &self.options{
            match option{
                Address(v) => write!(url, "address={}", v).expect("failed to write"),
                Port(v) => write!(url, "port={}", v).expect("failed to write"),
                StreamType(v) => write!(url, "stream={}", v).expect("failed to write"),
                StreamID(v) => write!(url, "sid={}", v).expect("failed to write"),
                ConnectionID(v) => write!(url, "CID={}", v).expect("failed to write"),
                PrincipalID(v) => write!(url, "PID={}", v).expect("failed to write"),
                NatType(v) => write!(url, "type={}", v).expect("failed to write"),
                NatMapping(v) => write!(url, "natm={}", v).expect("failed to write"),
                NatFiltering(v) => write!(url, "natf={}", v).expect("failed to write"),
                UPNP(v) => write!(url, "upnp={}", v).expect("failed to write"),
                RVConnectionID(v) => write!(url, "RVCID={}", v).expect("failed to write"),
                Platform(v) => write!(url, "pl={}", v).expect("failed to write"),
                PMP(v) => write!(url, "pmp={}", v).expect("failed to write"),
                PID(v) => write!(url, "PID={}", v).expect("failed to write"),
            }
            write!(url, ";").expect("failed to write");
        }

        url[0..url.len()-1].into()
    }
}

impl Display for StationUrl{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str: String = self.into();

        write!(f, "{}", str)
    }


}

impl RmcSerialize for StationUrl{
    fn deserialize(reader: &mut dyn Read) -> crate::rmc::structures::Result<Self> {
        let str = String::deserialize(reader)?;

        Self::try_from(str.as_str()).map_err(|_| StationUrlInvalid)
    }
    fn serialize(&self, writer: &mut dyn std::io::Write) -> crate::rmc::structures::Result<()> {
        let str: String = self.into();

        str.serialize(writer)
    }
}

impl Debug for StationUrl{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str: String = self.into();
        f.write_str(&str)
    }
}