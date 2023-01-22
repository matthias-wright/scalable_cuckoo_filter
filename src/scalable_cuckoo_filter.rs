use rand::{rngs::StdRng, Rng, SeedableRng};
use siphasher::sip::SipHasher13;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::cuckoo_filter::CuckooFilter;

/// Default Hasher.
pub type DefaultHasher = SipHasher13;

/// Default random number generator.
pub type DefaultRng = StdRng;

/// Builder for `ScalableCuckooFilter`.
#[derive(Debug)]
pub struct ScalableCuckooFilterBuilder<H = DefaultHasher, R = DefaultRng> {
    initial_capacity: usize,
    false_positive_probability: f64,
    entries_per_bucket: usize,
    max_kicks: usize,
    hasher: H,
    rng: R,
}
impl ScalableCuckooFilterBuilder<DefaultHasher> {
    /// Makes a new `ScalableCuckooFilterBuilder` instance.
    pub fn new() -> Self {
        ScalableCuckooFilterBuilder {
            initial_capacity: 100_000,
            false_positive_probability: 0.001,
            entries_per_bucket: 4,
            max_kicks: 512,
            hasher: SipHasher13::new(),
            rng: SeedableRng::from_entropy(),
        }
    }
}
impl<H: Hasher + Clone> ScalableCuckooFilterBuilder<H> {
    /// Sets the initial capacity (i.e., the number of estimated maximum items) of this filter.
    ///
    /// The default value is `100_000`.
    #[must_use]
    pub fn initial_capacity(mut self, capacity_hint: usize) -> Self {
        self.initial_capacity = capacity_hint;
        self
    }

    /// Sets the expected upper bound of the false positive probability of this filter.
    ///
    /// The default value is `0.001`.
    ///
    /// # Panics
    ///
    /// This method panics if `probability` is not a non-negative number smaller than or equal to `1.0`.
    #[must_use]
    pub fn false_positive_probability(mut self, probability: f64) -> Self {
        assert!(0.0 < probability && probability <= 1.0);
        self.false_positive_probability = probability;
        self
    }

    /// Sets the number of entries per bucket of this filter.
    ///
    /// The default value is `4`.
    #[must_use]
    pub fn entries_per_bucket(mut self, n: usize) -> Self {
        self.entries_per_bucket = n;
        self
    }

    /// Sets the maximum number of relocations in an insertion.
    ///
    /// If this limit exceeded, the filter will be expanded.
    ///
    /// The default value is `512`.
    #[must_use]
    pub fn max_kicks(mut self, kicks: usize) -> Self {
        self.max_kicks = kicks;
        self
    }

    /// Sets the hasher of this filter.
    ///
    /// The default value if `DefaultHasher::new()`.
    pub fn hasher<T: Hasher + Clone>(self, hasher: T) -> ScalableCuckooFilterBuilder<T> {
        ScalableCuckooFilterBuilder {
            initial_capacity: self.initial_capacity,
            false_positive_probability: self.false_positive_probability,
            entries_per_bucket: self.entries_per_bucket,
            max_kicks: self.max_kicks,
            hasher,
            rng: self.rng,
        }
    }

    /// Sets the random number generator of this filter.
    ///
    /// The default value is `rand::thread_rng()`.
    pub fn rng<T: Rng>(self, rng: T) -> ScalableCuckooFilterBuilder<H, T> {
        ScalableCuckooFilterBuilder {
            initial_capacity: self.initial_capacity,
            false_positive_probability: self.false_positive_probability,
            entries_per_bucket: self.entries_per_bucket,
            max_kicks: self.max_kicks,
            hasher: self.hasher,
            rng,
        }
    }

    /// Builds a `ScalableCuckooFilter` instance.
    pub fn finish<T: Hash + ?Sized>(self) -> ScalableCuckooFilter<T, H> {
        let mut filter = ScalableCuckooFilter {
            hasher: self.hasher,
            rng: self.rng,
            initial_capacity: self.initial_capacity,
            false_positive_probability: self.false_positive_probability,
            entries_per_bucket: self.entries_per_bucket,
            max_kicks: self.max_kicks,
            filters: Vec::new(),
            _item: PhantomData,
        };
        filter.grow();
        filter
    }
}
impl Default for ScalableCuckooFilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "serde_support")]
fn default_rng() -> StdRng {
    SeedableRng::from_entropy()
}

/// Scalable Cuckoo Filter.
#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct ScalableCuckooFilter<T: ?Sized, H = DefaultHasher> {
    #[cfg_attr(feature = "serde_support", serde(skip))]
    hasher: H,
    filters: Vec<CuckooFilter>,
    initial_capacity: usize,
    false_positive_probability: f64,
    entries_per_bucket: usize,
    max_kicks: usize,
    #[cfg_attr(feature = "serde_support", serde(skip, default = "default_rng"))]
    rng: StdRng,
    _item: PhantomData<T>,
}
impl<T: Hash + ?Sized> ScalableCuckooFilter<T> {
    /// Makes a new `ScalableCuckooFilter` instance.
    ///
    /// This is equivalent to the following expression:
    ///
    /// ```
    /// # use scalable_cuckoo_filter::{ScalableCuckooFilter, ScalableCuckooFilterBuilder};
    /// # let initial_capacity = 10;
    /// # let false_positive_probability = 0.1;
    /// # let _: ScalableCuckooFilter<()> =
    /// ScalableCuckooFilterBuilder::new()
    ///     .initial_capacity(initial_capacity)
    ///     .false_positive_probability(false_positive_probability)
    ///     .finish()
    /// # ;
    /// ```
    pub fn new(initial_capacity_hint: usize, false_positive_probability: f64) -> Self {
        ScalableCuckooFilterBuilder::new()
            .initial_capacity(initial_capacity_hint)
            .false_positive_probability(false_positive_probability)
            .finish()
    }
}
impl<T: Hash + ?Sized, H: Hasher + Clone> ScalableCuckooFilter<T, H> {
    /// Returns the approximate number of items inserted in this filter.
    pub fn len(&self) -> usize {
        self.filters.iter().map(|f| f.len()).sum()
    }

    /// Returns `true` if this filter contains no items, otherwise `false`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity (i.e., the upper bound of acceptable items count) of this filter.
    ///
    /// "capacity" is upper bound of the number of items can be inserted into the filter without resizing.
    pub fn capacity(&self) -> usize {
        self.filters.iter().map(|f| f.capacity()).sum()
    }

    /// Returns the number of bits being used for representing this filter.
    pub fn bits(&self) -> u64 {
        self.filters.iter().map(|f| f.bits()).sum()
    }

    /// Returns `true` if this filter may contain `item`, otherwise `false`.
    pub fn contains(&self, item: &T) -> bool {
        let item_hash = crate::hash(&self.hasher, item);
        self.filters
            .iter()
            .any(|f| f.contains(&self.hasher, item_hash))
    }

    /// Inserts `item` into this filter.
    ///
    /// If the current filter becomes full, it will be expanded automatically.
    pub fn insert(&mut self, item: &T) {
        let item_hash = crate::hash(&self.hasher, item);
        let last = self.filters.len() - 1;
        for filter in self.filters.iter().take(last) {
            if filter.contains(&self.hasher, item_hash) {
                return;
            }
        }

        self.filters[last].insert(&self.hasher, &mut self.rng, item_hash);
        if self.filters[last].is_nearly_full() {
            self.grow();
        }
    }

    /// Shrinks the capacity of this filter as much as possible.
    pub fn shrink_to_fit(&mut self) {
        for f in &mut self.filters {
            f.shrink_to_fit(&self.hasher, &mut self.rng);
        }
    }

    /// Removes `item` from this filter.
    pub fn remove(&mut self, item: &T) {
        let item_hash = crate::hash(&self.hasher, item);
        self.filters
            .iter_mut()
            .for_each(|f| f.remove(&self.hasher, item_hash));
    }

    fn grow(&mut self) {
        let capacity = self.initial_capacity * 2usize.pow(self.filters.len() as u32);
        let probability =
            self.false_positive_probability / 2f64.powi(self.filters.len() as i32 + 1);
        let fingerprint_bitwidth = ((1.0 / probability).log2()
            + ((2 * self.entries_per_bucket) as f64).log2())
        .ceil() as usize;
        let filter = CuckooFilter::new(
            fingerprint_bitwidth,
            self.entries_per_bucket,
            capacity,
            self.max_kicks,
        );
        self.filters.push(filter);
    }
}
impl<T: Hash + ?Sized, H: Hasher + Clone> Clone for ScalableCuckooFilter<T, H> {
    fn clone(&self) -> Self {
        ScalableCuckooFilter {
            hasher: self.hasher.clone(),
            filters: self.filters.clone(),
            initial_capacity: self.initial_capacity,
            false_positive_probability: self.false_positive_probability,
            entries_per_bucket: self.entries_per_bucket,
            max_kicks: self.max_kicks,
            rng: self.rng.clone(),
            _item: self._item,
        }
    }
}
impl<T: Hash + ?Sized, H: Hasher + Clone> PartialEq for ScalableCuckooFilter<T, H> {
    fn eq(&self, other: &Self) -> bool {
        self.filters.eq(&other.filters)
            && self.initial_capacity == other.initial_capacity
            && self.false_positive_probability == other.false_positive_probability
            && self.entries_per_bucket == other.entries_per_bucket
            && self.max_kicks == other.max_kicks
    }
}
impl<T: Hash + ?Sized, H: Hasher + Clone> Eq for ScalableCuckooFilter<T, H> {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let mut filter = ScalableCuckooFilter::new(1000, 0.001);
        assert!(filter.is_empty());
        assert_eq!(filter.bits(), 14_336);

        assert!(!filter.contains("foo"));
        filter.insert("foo");
        assert!(filter.contains("foo"));
    }

    #[test]
    fn insert_works() {
        use rand::{rngs::StdRng, SeedableRng};

        let mut seed = [0; 32];
        for i in 0..seed.len() {
            seed[i] = i as u8;
        }

        let rng: StdRng = SeedableRng::from_seed(seed);
        let mut filter = ScalableCuckooFilterBuilder::new()
            .initial_capacity(100)
            .false_positive_probability(0.00001)
            .rng(rng)
            .finish();
        for i in 0..10_000 {
            assert!(!filter.contains(&i));
            filter.insert(&i);
            assert!(filter.contains(&i));
        }
        assert_eq!(filter.len(), 10_000);
    }

    #[test]
    fn remove_works() {
        use rand::{rngs::StdRng, SeedableRng};

        let mut seed = [0; 32];
        for i in 0..seed.len() {
            seed[i] = i as u8;
        }

        let rng: StdRng = SeedableRng::from_seed(seed);
        let mut filter = ScalableCuckooFilterBuilder::new()
            .initial_capacity(100)
            .false_positive_probability(0.00001)
            .rng(rng)
            .finish();

        for i in 0..10_000 {
            filter.insert(&i);
        }
        for i in 0..10_000 {
            filter.remove(&i);
            assert!(!filter.contains(&i));
        }
    }

    #[test]
    fn shrink_to_fit_works() {
        let mut filter = ScalableCuckooFilter::new(1000, 0.001);
        for i in 0..100 {
            filter.insert(&i);
        }
        assert_eq!(filter.capacity(), 1024);
        assert_eq!(filter.bits(), 14336);

        filter.shrink_to_fit();
        for i in 0..100 {
            assert!(filter.contains(&i));
        }
        assert_eq!(filter.capacity(), 128);
        assert_eq!(filter.bits(), 1792);
    }

    #[cfg(feature = "serde_support")]
    use serde_json;
    #[test]
    #[cfg(feature = "serde_support")]
    fn serialize_dezerialize_works() {
        let mut filter = ScalableCuckooFilter::new(1000, 0.001);
        for i in 0..100 {
            filter.insert(&i);
        }
        filter.shrink_to_fit();
        let serialized = serde_json::to_string(&filter).unwrap();
        let deserialized: ScalableCuckooFilter<usize> = serde_json::from_str(&serialized).unwrap();
        for i in 0..100 {
            assert!(filter.contains(&i));
            assert!(deserialized.contains(&i));
        }
    }
}
