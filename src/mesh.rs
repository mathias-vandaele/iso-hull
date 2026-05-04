#[derive(Debug, Clone, Copy)]
pub(crate) struct Triangle {
    pub(crate) a: usize,
    pub(crate) b: usize,
    pub(crate) c: usize,
}

impl Triangle {
    pub(crate) fn edges(self) -> [Edge; 3] {
        [
            Edge::new(self.a, self.b),
            Edge::new(self.b, self.c),
            Edge::new(self.c, self.a),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Edge {
    pub(crate) u: usize,
    pub(crate) v: usize,
}

impl Edge {
    pub(crate) fn new(a: usize, b: usize) -> Self {
        if a <= b {
            Self { u: a, v: b }
        } else {
            Self { u: b, v: a }
        }
    }
}
