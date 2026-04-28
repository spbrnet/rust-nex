use log::{info, warn};
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
        let u32_len = self.data.len() as u32 - 1;
        (u32_len + 4).serialize(writer)?;
        u32_len.serialize(writer)?;
        writer.write_all(&self.data)?;

        Ok(())
    }
    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        let name = String::deserialize(reader)?;

        let size = u32::deserialize(reader)? as usize + 1;
        let mut buf = vec![0; size];
        reader.read_exact(&mut buf[..])?;
        let mut cursor = Cursor::new(&buf);
        let len2: u32 = cursor.read_struct(IS_BIG_ENDIAN)?;

        if len2 as usize + 3 != size {
            warn!("mismatched sizes on any: {} vs {}", size, len2 + 1);
        }

        info!("any data: {}", hex::encode(&buf));

        Ok(Any {
            name,
            data: (&buf[4..]).to_owned(),
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
    use std::io::Cursor;

    use crate::rmc::structures::{
        RmcSerialize,
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

        let any = Any::new(&sess).unwrap().to_data().unwrap();

        let sess2: MatchmakeSession = Any::deserialize(&mut Cursor::new(any))
            .unwrap()
            .try_get()
            .unwrap()
            .unwrap();

        assert_eq!(sess, sess2);
    }
}
