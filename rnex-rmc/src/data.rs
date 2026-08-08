use std::io::{Read, Write};

use crate::{
    config::FEATURE_HAS_STRUCT_HEADER,
    helpers, rmc_struct,
    serialization::{Result, RmcSerialize, RmcStruct, RmcStructInfo},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Data {}
impl RmcSerialize for Data {
    #[inline(always)]
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        rmc_struct::write_struct(
            writer,
            Self::version().unwrap(),
            helpers::len_of_write(|_| Ok(())),
            |_| Ok(()),
        )
    }
    #[inline(always)]
    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self> {
        rmc_struct::read_struct(reader, Self::version().unwrap(), move |_| Ok(Self {}))
    }
    fn serialize_write_size(&self) -> Result<u32> {
        Ok(if FEATURE_HAS_STRUCT_HEADER { 5 } else { 0 })
    }
    fn version() -> Option<u8> {
        Some(0)
    }
}

impl RmcStruct for Data {
    fn get_struct_info() -> &'static RmcStructInfo {
        static STRUCT_DATA: RmcStructInfo = RmcStructInfo {
            inheritors: ::std::sync::RwLock::new(::std::vec::Vec::new()),
            name: "Data",
        };
        &STRUCT_DATA
    }
}
