use std::{collections::HashSet, hash::Hash, str::FromStr, string::ToString};

use rnex_core::rmc::structures::RmcSerialize;

#[derive(Debug)]
struct StringSet<T: FromStr + ToString + Eq>(HashSet<T>)
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static;

impl<T: FromStr + ToString + Eq + Hash> PartialEq for StringSet<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().eq(&other.0)
    }
}

impl<T: FromStr + ToString + Eq + Hash> ToString for StringSet<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(ToString::to_string)
            .reduce(|a, b| format!("{}|{}", a, b))
            .unwrap_or(String::new())
    }
}

impl<T: FromStr + ToString + Eq + Hash> FromStr for StringSet<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    type Err = Box<dyn std::error::Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split("|")
                .filter(|v| !v.is_empty())
                .map(T::from_str)
                .try_fold(
                    HashSet::new(),
                    |mut a, b| -> Result<HashSet<T>, Self::Err> {
                        a.insert(b.map_err(Box::new)?);
                        Ok(a)
                    },
                )?,
        ))
    }
}

impl<T: FromStr + ToString + Eq + Hash> RmcSerialize for StringSet<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn deserialize(reader: &mut impl std::io::prelude::Read) -> super::Result<Self>
    where
        Self: Sized,
    {
        Self::from_str(&String::deserialize(reader)?).map_err(super::Error::Other)
    }
    fn serialize(&self, writer: &mut impl std::io::prelude::Write) -> super::Result<()> {
        self.to_string().serialize(writer)
    }
    fn serialize_write_size(&self) -> super::Result<u32> {
        self.to_string().serialize_write_size()
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::rmc::structures::string_set::StringSet;

    #[test]
    fn test() {
        let str_val = "0|100|200|10|110|210|20|120|220|30|130|230";
        let set: StringSet<u32> = StringSet::from_str(str_val).unwrap();
        let string_2 = set.to_string();
        let reset: StringSet<u32> = StringSet::from_str(&string_2).unwrap();

        for val in &set.0 {
            if !reset.0.contains(&val) {
                panic!("sets arent equivalent");
            }
        }

        let _: StringSet<u32> = StringSet::from_str("").unwrap();

        let _: StringSet<u32> = StringSet::from_str("10").unwrap();
    }
}
