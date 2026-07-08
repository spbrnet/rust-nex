use macros::RmcSerialize;

#[derive(RmcSerialize, Debug, Default, Clone)]
#[rmc_struct(0)]
pub struct ResultsRange {
    pub offset: u32,
    pub size: u32,
}

impl ResultsRange {
    pub fn make_from_list<T: Clone>(&self, list: &[T]) -> Vec<T> {
        let end = usize::max(list.len(), self.offset as usize + self.size as usize);

        list.get(self.offset as usize..end)
            .unwrap_or_default()
            .into()
    }
}
