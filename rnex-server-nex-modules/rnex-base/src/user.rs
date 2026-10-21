use std::net::SocketAddr;

use rnex_base_protos::{LocalBaseProtocol, secure::Secure, util::Utility};
use rnex_rmc::{any::Any, qbuffer::QBuffer, qresult::QResult, response::ErrorCode, rmc_struct};
use rnex_util::{
    PID, diagnostics,
    station_url::{StationUrl, UrlOptions, nat_types::PUBLIC},
};
use tokio::sync::RwLock;
use tracing::info;

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
            ) && (!cfg!(feature = "nx") || !matches!(v, UrlOptions::Platform(_)))
        });

        public_station.options.push(UrlOptions::Address(addr.ip()));
        public_station
            .options
            .push(UrlOptions::Port(if cfg!(feature = "nx") {
                20000
            } else {
                addr.port()
            }));
        public_station.options.push(UrlOptions::NatFiltering(0));
        public_station.options.push(UrlOptions::NatMapping(0));
        public_station
            .options
            .push(UrlOptions::NatType(if cfg!(feature = "nx") {
                11
            } else {
                3
            }));
        if cfg!(feature = "nx")
            && !public_station
                .options
                .iter()
                .any(|v| matches!(v, UrlOptions::TransportType(_)))
        {
            public_station.options.push(UrlOptions::TransportType(2));
        }

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
        if !cfg!(feature = "nx") {
            station.options.push(UrlOptions::ConnectionID(cid));
        }
    }

    Ok(vec![public_station])
}

#[rmc_struct(BaseProtocol)]
#[derive(Debug)]
pub struct BaseUser {
    // pub(crate) bm: PassthroughInitModule<BaseManager>,
    pub addr: SocketAddr,
    pub station_url: RwLock<Vec<StationUrl>>,
    #[cfg(feature = "splatoon2")]
    pub associated_ids: std::sync::Arc<
        RwLock<std::collections::HashMap<PID, Vec<rnex_base_protos::util::UniqueIdInfo>>>,
    >,
    pub pid: PID,
    pub cid: u32,
}

impl Secure for BaseUser {
    #[cfg(feature = "nx")]
    async fn test_connectivity(&self) -> Result<(), ErrorCode> {
        Ok(())
    }
    async fn register(
        &self,
        station_urls: Vec<StationUrl>,
    ) -> Result<(QResult, u32, StationUrl), ErrorCode> {
        let cid = self.cid;
        info!("station URLs in register: {:?}", station_urls);
        diagnostics::record(
            "station_register_in",
            format_args!(
                "pid={} cid={} address={} urls={station_urls:?}",
                self.pid, self.cid, self.addr
            ),
        );

        /*
         */

        let stations = get_station_urls(&station_urls, self.addr, self.pid, cid).await?;
        diagnostics::record(
            "station_register_out",
            format_args!("pid={} cid={cid} urls={stations:?}", self.pid),
        );

        let mut first = stations.first().unwrap().clone();
        if cfg!(feature = "nx") {
            first.options.retain(|v| {
                !matches!(
                    v,
                    UrlOptions::PrincipalID(_)
                        | UrlOptions::RVConnectionID(_)
                        | UrlOptions::ConnectionID(_)
                )
            });
        }

        let mut lock = self.station_url.write().await;

        *lock = stations;

        drop(lock);

        diagnostics::record(
            "station_register_response",
            format_args!("pid={} cid={} url={first:?}", self.pid, self.cid),
        );
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

        if cfg!(feature = "nx") {
            if !dest
                .options
                .iter()
                .any(|option| matches!(option, UrlOptions::PrincipalID(pid) if *pid == self.pid))
            {
                return Err(ErrorCode::Core_InvalidArgument);
            }
            let port = dest
                .options
                .iter()
                .find_map(|option| match option {
                    UrlOptions::Port(port) => Some(*port),
                    _ => None,
                })
                .ok_or(ErrorCode::Core_InvalidArgument)?;
            let mut public = dest.clone();
            public.options.retain(|option| {
                !matches!(
                    option,
                    UrlOptions::Address(_)
                        | UrlOptions::Port(_)
                        | UrlOptions::NatFiltering(_)
                        | UrlOptions::NatMapping(_)
                        | UrlOptions::NatType(_)
                        | UrlOptions::Platform(_)
                        | UrlOptions::PrincipalID(_)
                        | UrlOptions::RVConnectionID(_)
                        | UrlOptions::ConnectionID(_)
                        | UrlOptions::TransportType(_)
                )
            });
            public.options.extend([
                UrlOptions::Address(self.addr.ip()),
                UrlOptions::Port(port),
                UrlOptions::PrincipalID(self.pid),
                UrlOptions::RVConnectionID(self.cid),
                UrlOptions::TransportType(2),
                UrlOptions::NatFiltering(0),
                UrlOptions::NatMapping(0),
                UrlOptions::NatType(11),
            ]);
            *lock = vec![dest, public];
            diagnostics::record(
                "station_replace_out",
                format_args!("pid={} cid={} urls={lock:?}", self.pid, self.cid),
            );
            return Ok(());
        }

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

    async fn send_report(&self, _id: u32, _data: QBuffer) -> Result<(), ErrorCode> {
        Ok(())
    }
}

impl Utility for BaseUser {
    async fn acquire_nex_unique_id(&self) -> Result<u64, ErrorCode> {
        // splatoon 2 just kinda... needs this? idk.
        #[cfg(feature = "splatoon2")]
        return Ok(1_000_000_000_000_000 + self.pid as u64);
        #[cfg(not(feature = "splatoon2"))]
        Ok(rand::random())
    }

    #[cfg(feature = "nx")]
    async fn acquire_nex_unique_id_with_password(
        &self,
    ) -> Result<rnex_base_protos::util::UniqueIdInfo, ErrorCode> {
        #[cfg(feature = "splatoon2")]
        return Ok(rnex_base_protos::util::UniqueIdInfo {
            id: 1_000_000_000_000_000 + self.pid as u64,
            password: rand::random(),
        });
        #[cfg(not(feature = "splatoon2"))]
        Ok(rnex_base_protos::util::UniqueIdInfo {
            id: self.pid as u64,
            password: (self.pid as u64) ^ 0x4e45_585f_5057_4421,
        })
    }

    #[cfg(all(feature = "nx", not(feature = "splatoon2")))]
    async fn associate_nex_unique_id_with_my_principal_id(
        &self,
        _id: u64,
    ) -> Result<(), ErrorCode> {
        Ok(())
    }

    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    async fn associate_nex_unique_id_with_my_principal_id(
        &self,
        info: rnex_base_protos::util::UniqueIdInfo,
    ) -> Result<(), ErrorCode> {
        self.associated_ids
            .write()
            .await
            .insert(self.pid, vec![info]);
        Ok(())
    }

    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    async fn associate_nex_unique_ids_with_my_principal_id(
        &self,
        infos: Vec<rnex_base_protos::util::UniqueIdInfo>,
    ) -> Result<(), ErrorCode> {
        self.associated_ids.write().await.insert(self.pid, infos);
        Ok(())
    }

    #[cfg(feature = "nx")]
    async fn get_associated_nex_unique_id_with_my_principal_id(
        &self,
    ) -> Result<rnex_base_protos::util::UniqueIdInfo, ErrorCode> {
        #[cfg(feature = "splatoon2")]
        return Ok(self
            .associated_ids
            .read()
            .await
            .get(&self.pid)
            .and_then(|infos| infos.first())
            .cloned()
            .unwrap_or_default());
        #[cfg(not(feature = "splatoon2"))]
        self.acquire_nex_unique_id_with_password().await
    }

    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    async fn get_associated_nex_unique_ids_with_my_principal_id(
        &self,
    ) -> Result<Vec<rnex_base_protos::util::UniqueIdInfo>, ErrorCode> {
        Ok(self
            .associated_ids
            .read()
            .await
            .get(&self.pid)
            .cloned()
            .unwrap_or_default())
    }

    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    async fn acquire_tag_id(&self, nex_unique_ids: Vec<u64>) -> Result<u64, ErrorCode> {
        let mut ids = nex_unique_ids;
        ids.sort_unstable();
        let mut hash = 0xcbf29ce484222325_u64;
        for id in ids {
            for byte in id.to_le_bytes() {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        }
        Ok(hash)
    }

    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    async fn update_current_user(
        &self,
        param: rnex_base_protos::util::UpdateCurrentUserParam,
    ) -> Result<(), ErrorCode> {
        self.associated_ids
            .write()
            .await
            .insert(self.pid, vec![param.info]);
        Ok(())
    }

    async fn get_integer_settings(&self, index: u32) -> Result<Vec<(u16, i32)>, ErrorCode> {
        // this is... fucked. however i couldnt think of a better way.
        #[cfg(feature = "prudplite")]
        {
            let index_name = format!("NEX_UTILITY_SETTINGS_COUNT_{index}");
            let values_name = format!("NEX_UTILITY_SETTINGS_VALUES_{index}");
            let specific = std::env::var(&index_name).ok();
            let global = std::env::var("NEX_UTILITY_SETTINGS_COUNT").ok();
            let count = specific
                .as_deref()
                .or(global.as_deref())
                .and_then(|value| value.parse::<u16>().ok())
                .filter(|count| *count <= 4096)
                .unwrap_or(0);
            let values = std::env::var(if specific.is_some() {
                &values_name
            } else {
                "NEX_UTILITY_SETTINGS_VALUES"
            })
            .unwrap_or_default();
            let mut result: Vec<_> = (0..count).map(|key| (key, 0)).collect();
            for pair in values.split(',') {
                if let Some((key, value)) = pair.split_once('=') {
                    if let (Ok(key), Ok(value)) =
                        (key.trim().parse::<u16>(), value.trim().parse::<i32>())
                    {
                        if let Some(entry) = result.get_mut(key as usize) {
                            entry.1 = value;
                        }
                    }
                }
            }
            Ok(result)
        }
        #[cfg(not(feature = "prudplite"))]
        {
            let _ = index;
            Ok(vec![(0, 1), (1, 2), (2, 0), (3, 4)])
        }
    }

    #[cfg(feature = "nx")]
    async fn get_string_settings(&self, _index: u32) -> Result<Vec<(u16, String)>, ErrorCode> {
        Ok(Vec::new())
    }
}
