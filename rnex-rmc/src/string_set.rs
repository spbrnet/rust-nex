use std::{
    collections::HashSet,
    fmt::Debug,
    hash::Hash,
    io::{Read, Write},
    str::FromStr,
    string::ToString,
};

use crate::serialization::{Error, Result, RmcSerialize};

#[derive(Debug, Clone)]
pub struct StringSet<T: FromStr + ToString + Eq + Debug>(pub HashSet<T>)
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static;

impl<T: FromStr + ToString + Eq + Hash + Debug> PartialEq for StringSet<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().eq(&other.0)
    }
}

impl<T: FromStr + ToString + Eq + Hash + Debug> ToString for StringSet<T>
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

impl<T: FromStr + ToString + Eq + Hash + Debug> FromStr for StringSet<T>
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

impl<T: FromStr + ToString + Eq + Hash + Debug> RmcSerialize for StringSet<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn deserialize(reader: &mut (impl Read + ?Sized)) -> Result<Self>
    where
        Self: Sized,
    {
        Self::from_str(&String::deserialize(reader)?).map_err(Error::Other)
    }
    fn serialize(&self, writer: &mut (impl Write + ?Sized)) -> Result<()> {
        self.to_string().serialize(writer)
    }
    fn serialize_write_size(&self) -> Result<u32> {
        self.to_string().serialize_write_size()
    }
}

#[cfg(test)]
mod test {
    use crate::string_set::StringSet;
    use std::str::FromStr;

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
