#[derive(Default)]
struct NativeServicePage {
    nodes: std::collections::BTreeMap<(String, String), std::rc::Rc<NativeServiceNode>>,
    order: Vec<(String, String)>,
    constructing: std::collections::BTreeSet<(String, String)>,
}

impl NativeServicePage {
    const MAX_NODES: usize = 4096;

    fn node(&self, key: &(String, String)) -> Option<std::rc::Rc<NativeServiceNode>> {
        self.nodes.get(key).cloned()
    }

    fn insert(&mut self, key: (String, String), node: NativeServiceNode) {
        self.order.push(key.clone());
        self.nodes.insert(key, std::rc::Rc::new(node));
    }

    fn begin(&mut self, key: &(String, String)) -> Result<(), String> {
        if self.full() || self.constructing.len() >= 32 || !self.constructing.insert(key.clone()) {
            return Err(String::from(
                "Native provider dependency cycle or depth limit.",
            ));
        }
        Ok(())
    }

    fn end(&mut self, key: &(String, String)) {
        self.constructing.remove(key);
    }

    fn take_owner(&mut self, owner: &str) -> Vec<std::rc::Rc<NativeServiceNode>> {
        let keys = self
            .order
            .iter()
            .rev()
            .filter(|key| key.1 == owner)
            .cloned()
            .collect::<Vec<_>>();
        self.take_keys(&keys)
    }

    fn take_since(&mut self, checkpoint: usize) -> Vec<std::rc::Rc<NativeServiceNode>> {
        let keys = self.order[checkpoint..]
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>();
        self.take_keys(&keys)
    }

    fn take_keys(&mut self, keys: &[(String, String)]) -> Vec<std::rc::Rc<NativeServiceNode>> {
        let nodes = keys
            .iter()
            .filter_map(|key| self.nodes.remove(key))
            .collect();
        self.order.retain(|key| !keys.contains(key));
        nodes
    }

    fn full(&self) -> bool {
        self.nodes.len() >= Self::MAX_NODES
    }
}
