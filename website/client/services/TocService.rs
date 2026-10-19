#[Service]
#[NeedsAll]
struct TocService {
    entries: TacValue,
}

impl TocService {
    pub fn worth_showing(&self) -> bool {
        Self::has_several(&self.entries)
    }

    fn has_several(entries: &TacValue) -> bool {
        let TacValue::List(list) = entries else {
            return false;
        };
        list.len() > 1
    }
}
