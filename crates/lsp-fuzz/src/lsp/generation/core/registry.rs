#[derive(Debug, Default)]
pub struct GeneratorBag<G> {
    generators: Vec<G>,
}

impl<G> GeneratorBag<G> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            generators: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            generators: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, generator: G) {
        self.generators.push(generator);
    }

    pub fn push_weighted(&mut self, generator: G, weight: usize)
    where
        G: Clone,
    {
        if weight == 0 {
            return;
        }
        self.generators.reserve(weight);
        for _ in 0..weight {
            self.generators.push(generator.clone());
        }
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = G>,
    {
        self.generators.extend(iter);
    }

    #[must_use]
    pub fn finish(self) -> Vec<G> {
        self.generators
    }
}
