use std::{collections::HashMap, fmt::Debug, hash::Hash};

pub struct CollectionCounter<T>
where
    T: Eq + Hash + Debug,
{
    collection_counters: HashMap<T, u32>,
    expected_counters: HashMap<T, u32>,
}

impl<T> CollectionCounter<T>
where
    T: Eq + Hash + Debug,
{
    fn new<Iter>(collection: Iter) -> Self
    where
        Iter: Iterator<Item = T>,
    {
        let mut result = CollectionCounter {
            collection_counters: HashMap::new(),
            expected_counters: HashMap::new(),
        };

        for value in collection {
            result
                .collection_counters
                .entry(value)
                .and_modify(|v| *v += 1)
                .or_insert(1);
        }

        result
    }

    pub fn contains(&mut self, value: T, count: u32) -> &mut Self {
        self.expected_counters
            .entry(value)
            .and_modify(|v| *v += count)
            .or_insert(count);
        self
    }

    pub fn nothing_else(&mut self) {
        assert_eq!(self.collection_counters, self.expected_counters);
    }
}

pub trait CountableCollection<T>: Iterator<Item = T>
where
    T: Eq + Hash + Debug,
{
    fn to_collection_counter(&mut self) -> CollectionCounter<T> {
        CollectionCounter::new(self)
    }
}

impl<T, Iter> CountableCollection<T> for Iter
where
    Iter: Iterator<Item = T>,
    T: Eq + Hash + Debug,
{
}
