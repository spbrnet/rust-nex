#![allow(async_fn_in_trait)]

use rnex_rmc::{RmcSerialize, define_rmc_proto};
pub mod secure;
pub mod util;
use secure::{RawSecure, RawSecureInfo, RemoteSecure, Secure};
use util::{RawUtility, RawUtilityInfo, RemoteUtility, Utility};
use cfg_if::cfg_if;

#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(0)]
pub struct ResultsRange {
    pub offset: u32,
    pub size: u32,
}

cfg_if! {
    if #[cfg(feature = "v3-8-13")] {
        impl ResultsRange {
            pub fn make_from_list<T: Clone>(&self, list: &[T]) -> Vec<T> {
                let start = (self.offset as usize).min(list.len());
                let end = (start + self.size as usize).min(list.len());

                list[start..end].to_vec()
            }
        }
    } else {
        impl ResultsRange {
            pub fn make_from_list<T: Clone>(&self, list: &[T]) -> Vec<T> {
                let end = usize::max(list.len(), self.offset as usize + self.size as usize);

                list.get(self.offset as usize..end)
                    .unwrap_or_default()
                    .into()
            }
        }
    }
}

define_rmc_proto!(
    proto BaseProtocol{
        Secure,
        Utility,
    }
);
