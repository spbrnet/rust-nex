#![allow(async_fn_in_trait)]
use std::net::SocketAddr;
use std::sync::Weak;
use std::{
    any::{Any, TypeId, type_name},
    borrow::Cow,
    collections::HashMap,
    env,
    error::Error,
    fmt::{Debug, Display},
    io::{Read, Write},
    net::Ipv4Addr,
    ops::Deref,
    sync::{Arc, LazyLock, OnceLock},
};

pub use anyhow;
pub use paste;
pub use rnex_rmc as rmc;
use rnex_rmc::{RmcCallable, RmcConnection, RmcSerialize, serialization::RmcSerialize, util::PID};
pub use rnex_util as util;
pub use tokio;
pub use tracing;
use tracing::{Instrument, Level, error, instrument, span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, PartialEq, Eq, RmcSerialize)]
#[rmc_struct(0)]
pub struct ConnectionInitData {
    pub addr: SocketAddr,
    pub pid: PID,
}
#[derive(Debug, Default)]
pub struct ModuleHolder {
    modules: HashMap<TypeId, Arc<dyn Any>>,
}

// invariant: the [`OnceLock`] inside of the Arc MUST be initialized,
// otherwise creating this is concidered u.b.
#[derive(Debug)]
pub struct PassthroughInitModule<T: Any>(Arc<OnceLock<T>>);
#[derive(Debug)]
pub struct WeakPassthroughInitModule<T: Any>(Weak<OnceLock<T>>);

impl<T: Any> Clone for PassthroughInitModule<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Any> Clone for WeakPassthroughInitModule<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Any> Deref for PassthroughInitModule<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0.get().expect("invariant violated")
    }
}

impl<T: Any> AsRef<T> for PassthroughInitModule<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T: Any> PassthroughInitModule<T> {
    pub fn downgrade(this: &Self) -> WeakPassthroughInitModule<T> {
        WeakPassthroughInitModule(Arc::downgrade(&this.0))
    }
}
impl<T: Any> WeakPassthroughInitModule<T> {
    pub fn upgrade(&self) -> Option<PassthroughInitModule<T>> {
        self.0.upgrade().map(PassthroughInitModule)
    }
}

impl ModuleHolder {
    pub fn get_ref<T: Any>(&self) -> Option<Arc<OnceLock<T>>> {
        let module = self.modules.get(&TypeId::of::<T>())?;
        let Some(module) = module.downcast_ref::<Arc<OnceLock<T>>>() else {
            let type_name = type_name::<T>();
            error!(type_name, "module type inconsistency");
            return None;
        };

        Some(module.clone())
    }
    /// Gets a module passthrough reference to a specific module
    ///
    /// By using this function you acknowledge to not do anything on
    /// this object which invokes the deref() or as_ref() method until
    /// AFTER the object has been initialized inside the module holder.
    /// Doing so will result in a panic.
    pub fn get_ref_init_pt<T: Any>(&self) -> Option<PassthroughInitModule<T>> {
        self.get_ref::<T>().map(PassthroughInitModule)
    }
    pub fn create_empty_module_slot<T: Any>(&mut self) {
        self.modules
            .insert(TypeId::of::<T>(), Arc::new(OnceLock::<T>::new()));
    }
    /// Initialize a slot on the [`ModuleHolder`]
    ///
    /// This function allows you to initialize an empty slot in the module holder.
    /// An empty slot does not mean that the slot doesnt exist, as such you must have
    /// previously created the slot using [`create_empty_module_slot`]
    ///
    /// # Returns
    /// `Ok(_)` on success containing the wrapped value or
    /// `Err(_)` on failiure to set containing the value you passed in
    #[instrument]
    pub fn init_slot<T: Any + Debug>(&self, val: T) -> Result<PassthroughInitModule<T>, T> {
        let Some(slot) = self.get_ref() else {
            return Err(val);
        };
        if let Err(e) = slot.set(val) {
            return Err(e);
        };
        Ok(PassthroughInitModule(slot))
    }
}

pub trait RnexManager: Sized + 'static {
    type User: RmcCallable;
    type InitData: RmcSerialize;
    async fn init_new_user(
        this: PassthroughInitModule<Self>,
        mod_holder: &ModuleHolder,
        remote: &RmcConnection,
        init_data: &Self::InitData,
        weak_user: WeakPassthroughInitModule<Self::User>,
    ) -> Self::User;
    async fn post_init(_user: &Self::User) {}
}

pub trait RnexModule {
    type Manager: RnexManager;
    type InitError: Display;
    async fn create_manager(mod_holder: &ModuleHolder) -> Result<Self::Manager, Self::InitError>;
}

#[macro_export]
macro_rules! launch_rnex_module_server {
    {
    $init_ty:ty;
    $(
        $(#[$($tt:tt)*])*
        $module_type:ty
    ),* $(,)?} => {{
        use $crate::tracing::Instrument;
        use $crate::util::UnitPacketRead;
        #[allow(nonstandard_style)]
        $crate::with_setup(async || {
            $crate::paste::paste!{
            #[derive(Debug)]
            struct MultiEndpoint {
                $(
                    $(#[$($tt)*])*
                    [<user_ $module_type>]: $crate::PassthroughInitModule<
                        <<$module_type as $crate::RnexModule>::Manager as $crate::RnexManager>::User,
                    >
                ),*
            }
            }
            $crate::paste::paste!{
            #[derive(Debug)]
            struct MultiManager {
                $(
                    $(#[$($tt)*])*
                    [<manager_ $module_type>]: $crate::PassthroughInitModule<
                        <$module_type as $crate::RnexModule>::Manager,
                    >
                ),*
            }
            }
            impl $crate::rmc::RmcCallable for MultiEndpoint {
                async fn rmc_call(
                    &self,
                    responder: &$crate::util::SendingBufferConnection,
                    protocol_id: u16,
                    method_id: u32,
                    call_id: u32,
                    rest: &[u8],
                ) -> bool{
                    $(
                        $(#[$($tt)*])*
                        $crate::paste::paste!{
                            if self. [<user_ $module_type>]
                                .rmc_call(responder, protocol_id, method_id, call_id, rest)
                                .await{
                                    return true;
                                }
                        }
                    )*
                    false
                }
            }
            let managers = async {
                $crate::tracing::info!("creating module holder for managers");
                let mut holder = $crate::ModuleHolder::default();

                $(
                    $(#[$($tt)*])*
                    $crate::tracing::info!(
                        module_manager = ::std::any::type_name::<<$module_type as $crate::RnexModule>::Manager>(),
                        "creating module slot"
                    );
                    $(#[$($tt)*])*
                    holder.create_empty_module_slot::<<$module_type as $crate::RnexModule>::Manager>();
                )*

                $(
                    $(#[$($tt)*])*
                    $crate::tracing::info!(
                        module_manager = ::std::any::type_name::<<$module_type as $crate::RnexModule>::Manager>(),
                        "initializing and filling module slot"
                    );
                    $(#[$($tt)*])*
                    let $crate::paste::paste!{[<manager_ $module_type>]} = holder
                        .init_slot(
                            <$module_type as $crate::RnexModule>::create_manager(&holder).await?,
                        )
                        .expect("initialized manager twice");
                )*
                $crate::paste::paste!{
                    Ok::<_, $crate::anyhow::Error>(MultiManager{
                        $( $(#[$($tt)*])* [<manager_ $module_type>] ),*
                    })
                }
            }
            .instrument($crate::tracing::info_span!("initializing rnex server modules"))
            .await?;
            let socket = $crate::tokio::net::TcpListener::bind(::std::net::SocketAddrV4::new(*$crate::OWN_IP_PRIVATE, *$crate::SERVER_PORT))
                .instrument($crate::tracing::info_span!("binding to tcp socket"))
                .await?;
            async move {
                while let Ok((mut stream, _addr)) = socket.accept().await {
                    $crate::tracing::info!("new incoming connection");
                    async {
                        let Some(conn_data) = async  {
                            let buffer = match stream.read_buffer().await {
                                Ok(v) => v,
                                Err(e) => {
                                    $crate::tracing::error!(
                                        "an error ocurred whilst reading connection data buffer: {e:?}",
                                    );
                                    return None;
                                }
                            };

                            let user_connection_data =
                                <$init_ty as $crate::rmc::serialization::RmcSerialize>::deserialize(&mut ::std::io::Cursor::new(buffer));
                            match user_connection_data {
                                Ok(v) => Some(v),
                                Err(e) => {
                                    $crate::tracing::error!("an error ocurred whilst reading connection data: {:?}", e);
                                    return None;
                                }
                            }
                        }
                        .instrument($crate::tracing::info_span!("parse connection data from stream"))
                        .await
                        else {
                            return;
                        };

                            $crate::rmc::new_rmc_gateway_connection(stream.into(),
                                async |r| {
                                   $crate::tracing::info!("creating module holder for module users");
                                   let mut holder = $crate::ModuleHolder::default();

                                   $(
                                   $(#[$($tt)*])*
                                   $crate::tracing::info!(
                                       module_manager = ::std::any::type_name::<<<$module_type as $crate::RnexModule>::Manager as $crate::RnexManager>::User>(),
                                       "creating user module slot"
                                   );
                                   $(#[$($tt)*])*
                                   holder.create_empty_module_slot::<<<$module_type as $crate::RnexModule>::Manager as $crate::RnexManager>::User>();
                                   )*
                                   $(
                                   $(#[$($tt)*])*
                                   $crate::tracing::info!(
                                       module_manager = ::std::any::type_name::<<<$module_type as $crate::RnexModule>::Manager as $crate::RnexManager>::User>(),
                                       "initializing and filling user module slot if specified as present"
                                   );
                                   $crate::paste::paste!{
                                    $(#[$($tt)*])*
                                   let [<user_ $module_type>] = holder.init_slot($crate::RnexManager::init_new_user(managers.[<manager_ $module_type>].clone(), &holder, &r,  &conn_data, $crate::PassthroughInitModule::downgrade(&holder.get_ref_init_pt().expect("module slot should be initialized by now"))).await).expect("double init or uninit slot");
                                   }
                                   )*
                                   $crate::paste::paste!{
                                   ::std::sync::Arc::new(MultiEndpoint{

                                       $( $(#[$($tt)*])* [<user_ $module_type>] ),*
                                   })
                                   }
                                }
                            ).instrument($crate::tracing::info_span!("initializing user"))
                        .await;
                    }
                    .instrument($crate::tracing::info_span!("handeling new incoming connection"))
                    .await;
                }
            }
            .instrument($crate::tracing::info_span!("awaiting connections"))
            .await;
            Ok(())
        })
        .await;
    }};
}

pub fn rnex_release() -> String {
    let edition_piece = if let Some(e) = option_env!("EDITION") {
        format!("{}", e)
    } else {
        env!("FEATURESET").into()
    };

    format!(
        "rnex {} v{}({})",
        edition_piece,
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH")
    )
}

pub async fn with_setup(f: impl AsyncFnOnce() -> anyhow::Result<()>) {
    println!("setting up logger and dotenv");
    dotenv::dotenv().ok();
    let _maybe_sentry = if let Ok(sentry_url) = std::env::var("SENTRY_URL") {
        Some(sentry::init((
            sentry_url,
            sentry::ClientOptions {
                release: Some(Cow::Owned(rnex_release())),
                send_default_pii: true,
                ..Default::default()
            },
        )))
    } else {
        None
    };
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry::integrations::tracing::layer())
        .try_init()
        .expect("failed to init tracing subscriber");

    if let Err(res) = f().instrument(span!(Level::INFO, "main")).await {
        error!("fatal server error: {res}");
    };

    /*ctrlc::set_handler(||{
        FORCE_EXIT.call_once_force(|_|{
            println!("attempting exit");
        });
    }).unwrap();*/
}

const IP_REQ_SERVICE_URLS: &[(&str, &str, &str)] = &[
    ("ipinfo.io:80", "ipinfo.io", "/ip"),
    ("api.ipify.org:80", "api.ipify.org", "/"),
    // preresolved
    ("34.117.59.81:80", "ipinfo.io", "/ip"),
    ("104.26.13.205:80", "api.ipify.org", "/"),
    ("172.67.74.152:80", "api.ipify.org", "/"),
    ("104.26.12.205:80", "api.ipify.org", "/"),
];

/*
cfg_if! {
    if #[cfg(feature = "database-support")] {
        use std::sync::{LazyLock, OnceLock};
        use sqlx::postgres::PgPool;
        pub static RNEX_DATABASE_URL: LazyLock<String> = LazyLock::new(|| {
            std::env::var("RNEX_DATABASE_URL")
                .expect("RNEX_DATABASE_URL must be set")
        });

        pub static DB_POOL: OnceLock<PgPool> = OnceLock::new();

        pub fn get_db() -> &'static PgPool {
            DB_POOL.get().expect("db_pool not initialized")
        }
    }
}
cfg_if! {
    if #[cfg(feature = "datastore")]{
        pub static RNEX_DATASTORE_S3_ENDPOINT: LazyLock<String> = LazyLock::new(|| {
            std::env::var("RNEX_DATASTORE_S3_ENDPOINT")
                .expect("RNEX_DATASTORE_S3_ENDPOINT must be set")
        });
        pub static RNEX_DATASTORE_S3_BUCKET: LazyLock<String> = LazyLock::new(|| {
            std::env::var("RNEX_DATASTORE_S3_BUCKET")
                .expect("RNEX_DATASTORE_S3_BUCKET must be set")
        });
    }
}*/

pub fn try_to_log<R, E: Display>(fun: impl FnOnce() -> Result<R, E>) -> Option<R> {
    match fun() {
        Ok(v) => Some(v),
        Err(e) => {
            error!("{e}");
            None
        }
    }
}

pub fn try_get_ip() -> Option<Ipv4Addr> {
    for url in IP_REQ_SERVICE_URLS {
        println!("trying to get ip via: {:?}", url);
        if let Some(v) = try_to_log::<_, Box<dyn Error>>(|| {
            let mut stream = std::net::TcpStream::connect(url.0)?;
            stream.write_all(
                format!(
                    "GET {} HTTP/1.0
Host: {}
User-Agent: RNEX
Accept: */*

",
                    url.2, url.1
                )
                .as_str()
                .as_bytes(),
            )?;
            let mut data = vec![];
            stream.read_to_end(&mut data)?;
            let string = String::from_utf8(data)?;
            let (_, ip) = string
                .split_once("\r\n\r\n")
                .ok_or("unable to get ip from response")?;
            Ok(ip.parse()?)
        }) {
            return Some(v);
        }
    }
    None
}

pub static OWN_IP_PRIVATE: LazyLock<Ipv4Addr> = LazyLock::new(|| {
    env::var("RNEX_SERVER_IP")
        .ok()
        .map(|s| s.parse().expect("invalid ip address"))
        .unwrap_or(Ipv4Addr::UNSPECIFIED)
});

pub static OWN_IP_PUBLIC: LazyLock<Ipv4Addr> = LazyLock::new(|| {
    env::var("RNEX_SERVER_IP_PUBLIC")
        .ok()
        .map(|s| s.parse().expect("invalid ip address"))
        .unwrap_or_else(|| try_get_ip().unwrap())
});

pub static SERVER_PORT: LazyLock<u16> = LazyLock::new(|| {
    env::var("RNEX_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
});
