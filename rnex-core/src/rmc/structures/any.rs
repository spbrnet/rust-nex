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

        // also length ?
        let _len2: u32 = reader.read_struct(IS_BIG_ENDIAN)?;
        let data = Vec::deserialize(reader)?;

        Ok(Any { name, data })
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
        RmcSerialize,
        any::Any,
        matchmake::{Gathering, MatchmakeSession},
    };

    #[test]
    fn test() {
        let any = Any {
            name: "MatchmakeSession".into(),
            data: [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 2, 0, 98, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0,
                0, 0, 0, 0, 0, 20, 0, 68, 111, 111, 114, 115, 32, 70, 114, 105, 101, 110, 100, 32,
                73, 110, 118, 105, 116, 101, 0, 0, 0, 0, 0, 6, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
            ]
            .into(),
        };

        println!("{}", hex::encode(&any.data));

        let _: MatchmakeSession = any.try_get().unwrap().unwrap();
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
            #[cfg(feature = "v3-5-0")]
            progress_score: 0,
            session_key: [].into(),
        };

        let any = Any::new(&sess).unwrap();

        let sess2: MatchmakeSession = any.try_get().unwrap().unwrap();

        assert_eq!(sess, sess2)
    }
}
