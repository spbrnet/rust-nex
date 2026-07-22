use std::{
    fmt::{Debug, Display, Formatter},
    net::IpAddr,
    str::FromStr,
};

use thiserror::Error;
use tracing::error;

use crate::PID;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Type {
    UDP,
    PRUDP,
    PRUDPS,
}

pub mod nat_types {
    pub const BEHIND_NAT: u8 = 1;
    pub const PUBLIC: u8 = 2;
}

#[derive(Clone, Eq, PartialEq)]
pub enum UrlOptions {
    Address(IpAddr),
    Port(u16),
    StreamType(u8),
    StreamID(u8),
    ConnectionID(u32),
    ProbeInit(u32),
    PrincipalID(PID),
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
pub struct StationUrl {
    pub url_type: Type,
    pub options: Vec<UrlOptions>,
}

impl StationUrl {
    pub fn read_options(options: &str) -> Option<Vec<UrlOptions>> {
        let mut options_out = Vec::new();

        for option in options.split(';') {
            if option == "" {
                continue;
            }
            let mut option_parts = option.split('=');
            let option_name = option_parts.next()?.to_ascii_lowercase();
            let option_value = option_parts.next()?;

            use UrlOptions::*;

            match option_name.as_ref() {
                "address" => options_out.push(Address(option_value.parse().ok()?)),
                "port" => options_out.push(Port(option_value.parse().ok()?)),
                "natf" => options_out.push(NatFiltering(option_value.parse().ok()?)),
                "natm" => options_out.push(NatMapping(option_value.parse().ok()?)),
                "sid" => options_out.push(StreamID(option_value.parse().ok()?)),
                "upnp" => options_out.push(UPNP(option_value.parse().ok()?)),
                "type" => options_out.push(NatType(option_value.parse().ok()?)),
                "stream" => options_out.push(StreamType(option_value.parse().ok()?)),
                "RVCID" => options_out.push(RVConnectionID(option_value.parse().ok()?)),
                "rvcid" => options_out.push(RVConnectionID(option_value.parse().ok()?)),
                "CID" => options_out.push(ConnectionID(option_value.parse().ok()?)),
                "cid" => options_out.push(ConnectionID(option_value.parse().ok()?)),
                "pl" => options_out.push(Platform(option_value.parse().ok()?)),
                "pmp" => options_out.push(PMP(option_value.parse().ok()?)),
                "pid" => options_out.push(PID(option_value.parse().ok()?)),
                "PID" => options_out.push(PID(option_value.parse().ok()?)),
                "probeinit" => options_out.push(ProbeInit(option_value.parse().ok()?)),
                _ => {
                    error!("unimplemented option type, skipping: {}", option_name);
                }
            }
        }

        Some(options_out)
    }
}

// todo: add more specific error messages to parsing
#[derive(Error, Debug)]
#[error("failed to parse station url")]
pub struct StationUrlParseError;

impl FromStr for StationUrl {
    type Err = StationUrlParseError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (url_type, options) = value.split_at(value.find(":/").ok_or(StationUrlParseError)?);

        let options = &options[2..];

        use Type::*;

        let url_type = match url_type {
            "udp" => UDP,
            "prudp" => PRUDP,
            "prudps" => PRUDPS,
            _ => return Err(StationUrlParseError),
        };

        let options = Self::read_options(options).ok_or(StationUrlParseError)?;

        Ok(Self { url_type, options })
    }
}

impl Display for StationUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Type::*;
        let url_type_str = match self.url_type {
            UDP => "udp:/",
            PRUDP => "prudp:/",
            PRUDPS => "prudps:/",
        };
        write!(f, "{}", url_type_str)?;

        use UrlOptions::*;
        for option in &self.options {
            match option {
                Address(v) => write!(f, "address={}", v)?,
                Port(v) => write!(f, "port={}", v)?,
                StreamType(v) => write!(f, "stream={}", v)?,
                StreamID(v) => write!(f, "sid={}", v)?,
                ConnectionID(v) => write!(f, "CID={}", v)?,
                PrincipalID(v) => write!(f, "PID={}", v)?,
                NatType(v) => write!(f, "type={}", v)?,
                NatMapping(v) => write!(f, "natm={}", v)?,
                NatFiltering(v) => write!(f, "natf={}", v)?,
                UPNP(v) => write!(f, "upnp={}", v)?,
                RVConnectionID(v) => write!(f, "RVCID={}", v)?,
                Platform(v) => write!(f, "pl={}", v)?,
                PMP(v) => write!(f, "pmp={}", v)?,
                PID(v) => write!(f, "PID={}", v)?,
                ProbeInit(v) => write!(f, "probeinit={}", v)?,
            }
            write!(f, ";")?;
        }
        Ok(())
    }
}

impl<'a> Into<String> for &'a StationUrl {
    fn into(self) -> String {
        let url = self.to_string();

        url[0..url.len() - 1].into()
    }
}

impl Debug for StationUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str: String = self.into();
        f.write_str(&str)
    }
}
