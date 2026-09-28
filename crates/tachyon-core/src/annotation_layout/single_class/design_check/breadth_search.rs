#![allow(clippy::needless_bitwise_bool)]

pub(super) struct BreadthSearch<'g> {
    edges: &'g [Vec<usize>],
    start: usize,
    queue: Vec<usize>,
    parents: Vec<Option<usize>>,
    closing: Option<usize>,
}

impl<'g> BreadthSearch<'g> {
    pub(super) fn cycle(edges: &'g [Vec<usize>], start: usize) -> Option<Vec<usize>> {
        let mut search = Self {
            edges,
            start,
            queue: vec![start],
            parents: vec![None; edges.len()],
            closing: None,
        };
        search.walk();
        search.closing.map(|last| search.path_to(last))
    }

    fn walk(&mut self) {
        let mut at = 0;
        while (at < self.queue.len()) & self.closing.is_none() {
            self.expand(self.queue[at]);
            at += 1;
        }
    }

    fn expand(&mut self, node: usize) {
        for &next in &self.edges[node] {
            self.step(node, next);
        }
    }

    fn step(&mut self, node: usize, next: usize) {
        let closes = next == self.start;
        self.closing = self.closing.or(closes.then_some(node));
        let fresh = (next > self.start) & self.parents[next].is_none();
        if fresh {
            self.parents[next] = Some(node);
            self.queue.push(next);
        }
    }

    fn path_to(&self, last: usize) -> Vec<usize> {
        let mut path = vec![self.start, last];
        let mut at = last;
        while let Some(parent) = self.parents[at] {
            path.insert(1, parent);
            at = parent;
        }
        path.remove(1);
        path.push(self.start);
        path
    }
}
