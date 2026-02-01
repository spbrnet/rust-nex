use rnex_core::prudp::station_url::StationUrl;
use rnex_core::prudp::station_url::UrlOptions::{
    Address, NatFiltering, NatMapping, NatType, Port, PrincipalID, RVConnectionID,
};
use rnex_core::prudp::station_url::nat_types::PUBLIC;
use rnex_core::rmc::response::ErrorCode::Core_Exception;

use rnex_core::prudp::socket_addr::PRUDPSockAddr;
use rnex_core::rmc::response::ErrorCode;

pub async fn get_station_urls(
    station_urls: &[StationUrl],
    addr: PRUDPSockAddr,
    pid: u32,
    cid: u32,
) -> Result<Vec<StationUrl>, ErrorCode> {
    let mut public_station: Option<StationUrl> = None;
    let mut private_station: Option<StationUrl> = None;

    for station in station_urls {
        let is_public = station.options.iter().any(|v| {
            if let NatType(v) = v {
                if *v & PUBLIC != 0 {
                    return true;
                }
            }
            false
        });

        let Some(nat_filtering) = station.options.iter().find_map(|v| match v {
            NatFiltering(v) => Some(v),
            _ => None,
        }) else {
            return Err(Core_Exception);
        };

        let Some(nat_mapping) = station.options.iter().find_map(|v| match v {
            NatMapping(v) => Some(v),
            _ => None,
        }) else {
            return Err(Core_Exception);
        };

        if !is_public || (*nat_filtering == 0 && *nat_mapping == 0) {
            private_station = Some(station.clone());
        }

        if is_public {
            public_station = Some(station.clone());
        }
    }

    let Some(mut private_station) = private_station else {
        return Err(Core_Exception);
    };

    let mut public_station = if let Some(public_station) = public_station {
        public_station
    } else {
        let mut public_station = private_station.clone();

        public_station.options.retain(|v| match v {
            Address(_) | Port(_) | NatFiltering(_) | NatMapping(_) | NatType(_) => false,
            _ => true,
        });

        public_station
            .options
            .push(Address(*addr.regular_socket_addr.ip()));
        public_station
            .options
            .push(Port(addr.regular_socket_addr.port()));
        public_station.options.push(NatFiltering(0));
        public_station.options.push(NatMapping(0));
        public_station.options.push(NatType(3));

        public_station
    };

    let both = [&mut public_station, &mut private_station];

    for station in both {
        station.options.retain(|v| match v {
            PrincipalID(_) | RVConnectionID(_) => false,
            _ => true,
        });

        station.options.push(PrincipalID(pid));
        station.options.push(RVConnectionID(cid));
    }

    Ok(vec![public_station])
}
