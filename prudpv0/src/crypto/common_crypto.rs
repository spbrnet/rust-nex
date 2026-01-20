trait IterExtra: Iterator {
    fn sum_wrapping_u8(&mut self) -> u8
    where
        Self::Item: Into<u8>;

    fn sum_wrapping_u32(&mut self) -> u32
    where
        Self::Item: Into<u32>;
}

impl<T: Iterator> IterExtra for T {
    fn sum_wrapping_u8(&mut self) -> u8
    where
        Self::Item: Into<u8>,
    {
        let mut sum = 0u8;
        for v in self {
            let val: u8 = v.into();
            sum = sum.wrapping_add(val);
        }
        sum
    }
    fn sum_wrapping_u32(&mut self) -> u32
    where
        Self::Item: Into<u32>,
    {
        let mut sum = 0u32;
        for v in self {
            let val: u32 = v.into();
            sum = sum.wrapping_add(val);
        }
        sum
    }
}

#[inline(always)]
pub fn common_checksum(access_key: &str, data: &[u8]) -> u8 {
    let leftover = data.len() % 4;
    let word_sum = bytemuck::cast_slice::<_, u32>(&data[..data.len() - leftover])
        .iter()
        .copied()
        .sum_wrapping_u32();

    let checksum = access_key.as_bytes().iter().copied().sum_wrapping_u8();
    let checksum = checksum.wrapping_add(
        (&data[data.len() - leftover..])
            .iter()
            .copied()
            .sum_wrapping_u8(),
    );
    let checksum = checksum.wrapping_add(word_sum.to_ne_bytes().into_iter().sum_wrapping_u8());

    checksum
}
