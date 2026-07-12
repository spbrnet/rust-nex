use std::net::SocketAddr;

use rnex_base_protos::{LocalBaseProtocol, secure::Secure, util::Utility};
use rnex_rmc::{any::Any, qresult::QResult, response::ErrorCode, rmc_struct};
use rnex_server::PassthroughInitModule;
use rnex_util::{
    PID,
    station_url::{StationUrl, UrlOptions, nat_types::PUBLIC},
};
use tokio::sync::{Mutex, RwLock};
use tracing::info;

use crate::BaseManager;

pub async fn get_station_urls(
    station_urls: &[StationUrl],
    addr: SocketAddr,
    pid: PID,
    cid: u32,
) -> Result<Vec<StationUrl>, ErrorCode> {
    let mut public_station: Option<StationUrl> = None;
    let mut private_station: Option<StationUrl> = None;

    for station in station_urls {
        let is_public = station.options.iter().any(|v| {
            if let UrlOptions::NatType(v) = v
                && *v & PUBLIC != 0
            {
                return true;
            }
            false
        });

        let Some(nat_filtering) = station.options.iter().find_map(|v| match v {
            UrlOptions::NatFiltering(v) => Some(v),
            _ => None,
        }) else {
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Some(nat_mapping) = station.options.iter().find_map(|v| match v {
            UrlOptions::NatMapping(v) => Some(v),
            _ => None,
        }) else {
            return Err(ErrorCode::Core_InvalidArgument);
        };

        if !is_public || (*nat_filtering == 0 && *nat_mapping == 0) {
            private_station = Some(station.clone());
        }

        if is_public {
            public_station = Some(station.clone());
        }
    }

    let Some(mut private_station) = private_station else {
        return Err(ErrorCode::Core_InvalidArgument);
    };

    let mut public_station = if let Some(public_station) = public_station {
        public_station
    } else {
        let mut public_station = private_station.clone();

        public_station.options.retain(|v| {
            !matches!(
                v,
                UrlOptions::Address(_)
                    | UrlOptions::Port(_)
                    | UrlOptions::NatFiltering(_)
                    | UrlOptions::NatMapping(_)
                    | UrlOptions::NatType(_)
            )
        });

        public_station.options.push(UrlOptions::Address(addr.ip()));
        public_station.options.push(UrlOptions::Port(addr.port()));
        public_station.options.push(UrlOptions::NatFiltering(0));
        public_station.options.push(UrlOptions::NatMapping(0));
        public_station.options.push(UrlOptions::NatType(3));

        public_station
    };

    let both = [&mut public_station, &mut private_station];

    for station in both {
        station.options.retain(|v| {
            !matches!(
                v,
                UrlOptions::PrincipalID(_)
                    | UrlOptions::RVConnectionID(_)
                    | UrlOptions::ConnectionID(_)
            )
        });

        station.options.push(UrlOptions::PrincipalID(pid));
        station.options.push(UrlOptions::RVConnectionID(cid));
        station.options.push(UrlOptions::ConnectionID(cid));
    }

    Ok(vec![public_station])
}

#[rmc_struct(BaseProtocol)]
#[derive(Debug)]
pub struct BaseUser {
    pub(crate) bm: PassthroughInitModule<BaseManager>,
    pub addr: SocketAddr,
    pub station_url: RwLock<Vec<StationUrl>>,
    pub pid: PID,
    pub cid: u32,
}

impl Secure for BaseUser {
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        let cid = self.cid;
        println!("{:?}", station_urls);

        /*
        let mut users = self.matchmake_manager.users.write().await;
        users.insert(cid, self.this.clone());
        drop(users);
        let mut users = self.matchmake_manager.users_by_pid.write().await;
        users.insert(self.pid, self.this.clone());
        drop(users);
        */

        let stations = get_station_urls(&station_urls, self.addr, self.pid, cid).await?;

        let first = stations.first().unwrap().clone();

        let mut lock = self.station_url.write().await;

        *lock = stations;

        drop(lock);

        Ok((QResult::success(ErrorCode::Core_Unknown), self.cid, first))
    }

    async fn register_ex(
        &self,
        station_urls: Vec<StationUrl>,
        _data: Any,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        self.register(station_urls).await
    }

    async fn replace_url(&self, target_url: StationUrl, dest: StationUrl) -> Result<(), ErrorCode> {
        let mut lock = self.station_url.write().await;
        info!("target URL: {:?}", target_url);
        info!("dest URL: {:?}", dest);

        let Some(target_addr) = target_url
            .options
            .iter()
            .find(|v| matches!(v, UrlOptions::Address(_)))
        else {
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Some(target_port) = target_url
            .options
            .iter()
            .find(|v| matches!(v, UrlOptions::Port(_)))
        else {
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Some(replacement_target) = lock.iter_mut().find(|url| {
            url.options.iter().any(|o| o == target_addr)
                && url.options.iter().any(|o| o == target_port)
        }) else {
            //probably internal ip
            return Ok(());
        };
        *replacement_target = dest;

        drop(lock);

        Ok(())
    }
}

impl Utility for BaseUser {
    async fn acquire_nex_unique_id(&self) -> Result<u64, ErrorCode> {
        return Ok(rand::random());
    }

    async fn get_integer_settings(&self, _index: u32) -> Result<Vec<(u16, i32)>, ErrorCode> {
        Ok(vec![(0, 1), (1, 2), (2, 0), (3, 4)])
    }
}
