type SearchableFn<T> = dyn FnMut(&&T, &str) -> bool;

pub struct Searchable<T>
where
    T: Clone + std::fmt::Debug,
{
    vec: Vec<T>,

    filter: Box<SearchableFn<T>>,
    filtered: Vec<T>,
}

impl<T> std::fmt::Debug for Searchable<T>
where
    T: Clone + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Searchable")
            .field("vec_len", &self.vec.len())
            .field("filtered_len", &self.filtered.len())
            .field("vec", &self.vec)
            .field("filtered", &self.filtered)
            .finish_non_exhaustive()
    }
}

impl<T> Searchable<T>
where
    T: Clone + std::fmt::Debug,
{
    #[must_use]
    pub fn new<P>(vec: Vec<T>, search_value: &str, predicate: P) -> Self
    where
        P: FnMut(&&T, &str) -> bool + 'static,
    {
        let mut searchable = Self {
            vec,

            filter: Box::new(predicate),
            filtered: Vec::new(),
        };
        searchable.search(search_value);
        searchable
    }

    pub fn search(&mut self, value: &str) {
        if value.is_empty() {
            self.filtered.clone_from(&self.vec);
            return;
        }

        self.filtered = self
            .vec
            .iter()
            .filter(|host| (self.filter)(host, value))
            .cloned()
            .collect();
    }

    #[allow(clippy::must_use_candidate)]
    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    #[allow(clippy::must_use_candidate)]
    pub fn is_empty(&self) -> bool {
        self.filtered.is_empty()
    }

    pub fn non_filtered_iter(&self) -> std::slice::Iter<T> {
        self.vec.iter()
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.filtered.iter()
    }
}

impl<'a, T> IntoIterator for &'a Searchable<T>
where
    T: Clone + std::fmt::Debug,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.filtered.iter()
    }
}

impl<T> std::ops::Index<usize> for Searchable<T>
where
    T: Clone + std::fmt::Debug,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.filtered[index]
    }
}
