use log::warn;
use rnex_core::rmc::structures::{Result, RmcSerialize};
use std::io::{Cursor, Read, Write};
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

#[derive(Debug, Default, Clone)]
pub struct Any {
    pub name: String,
    pub data: Vec<u8>,
}

impl RmcSerialize for Any {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        self.name.serialize(writer)?;

        let u32_len = self.data.len() as u32;
        (u32_len + 4).serialize(writer)?;
        self.data.serialize(writer)?;

        Ok(())
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let name = String::deserialize(reader)?;

        let data = Vec::deserialize(reader)?;
        let mut cursor = Cursor::new(&data);
        // also length ?
        let len2: u32 = cursor.read_struct(IS_BIG_ENDIAN)?;

        if len2 as usize != data.len().overflowing_sub(4).0 {
            warn!(
                "mismatched sizes on any: {} vs {}",
                data.len().overflowing_sub(4).0,
                len2
            );
        }

        Ok(Any {
            name,
            data: (&data[4..]).to_owned(),
        })
    }
}

impl Any {
    pub fn try_get<T: RmcSerialize>(&self) -> Option<Result<T>> {
        if self.name != T::name() {
            return None;
        }
        return Some(T::deserialize(&mut Cursor::new(&self.data[..])));
    }
    pub fn new<T: RmcSerialize>(val: &T) -> Result<Self> {
        return Ok(Self {
            name: T::name().to_owned(),
            data: val.to_data()?,
        });
    }
}

#[cfg(test)]
mod test {
    use crate::rmc::structures::{
        any::Any,
        matchmake::{Gathering, MatchmakeSession},
    };

    #[test]
    fn test() {
        let sess = MatchmakeSession {
            gathering: Gathering {
                self_gid: 0,
                owner_pid: 0,
                host_pid: 0,
                minimum_participants: 2,
                maximum_participants: 2,
                participant_policy: 98,
                policy_argument: 0,
                flags: 32,
                state: 0,
                description: "Doors Friend Invite".into(),
            },
            gamemode: 0,
            attributes: [2, 3, 0, 0, 0, 0].into(),
            open_participation: false,
            matchmake_system_type: 2,
            application_buffer: [1, 2, 3].into(),
            participation_count: 0,
            progress_score: 0,
            session_key: [].into(),
        };

        let any = Any::new(&sess).unwrap();

        let sess2: MatchmakeSession = any.try_get().unwrap().unwrap();

        assert_eq!(sess, sess2);
    }
}
