#![cfg_attr(not(test), no_std)]

use core::hash::{Hash, Hasher};
use rustc_hash::FxHasher;

/// There is no support for compile time generic expressions in stable rust
/// So this helper macro initializes smol map with a slot bitmap with the necessary number of words
/// The slot bitmap allows us to perform occupancy queries
#[macro_export]
macro_rules! smol_map {
    ($k:ty, $v:ty, $n:expr) => {
        $crate::SmolMap::<$k, $v, { $n }, { ($n + 63) / 64 }>::new()
    };
}

/// A stack allocated map  for copy types
#[derive(Debug)]
pub struct SmolMap<K, V, const N: usize, const M: usize>
where
    K: Copy + Default + Hash + Eq,
    V: Copy + Default,
{
    slots: [u64; M],
    pub entries: [(K, V); N],
}

impl<K,V, const N: usize, const M: usize> Default for SmolMap<K,V,N,M> 
where
    K: Copy + Default + Hash + Eq,
    V: Copy + Default,
{
    fn default() -> Self {
        Self { slots: [0;M], entries: [(K::default(),V::default());N] }
    }
}

impl<K, V, const N: usize, const M: usize> SmolMap<K, V, N, M>
where
    K: Copy + Default + Hash + Eq,
    V: Copy + Default,
{
    // Checks the occupancy of an index in the bitmask
    #[inline]
    fn is_occupied(&self, i: usize) -> bool {
        ((self.slots[i / 64] >> (i % 64)) & 1) == 1
    }

    // Sets the bit in the bimask corresponding to the entry
    #[inline]
    fn set_occupied(&mut self, i: usize) {
        self.slots[i / 64] |= 1 << (i % 64);
    }

    #[inline]
    fn set_vacant(&mut self, i:usize) {
        self.slots[i/64] &= !(1 << (i % 64)); 
    }

    // Returns the index at which 
    #[inline]
    fn get_hash_idx(k: K) -> usize {
        let mut hasher = FxHasher::default();
        k.hash(&mut hasher);

        hasher.finish() as usize % N
    }

    /// returns the length of the entries array `N`
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Creates a new map intialized with default values.
    #[inline]
    pub fn new() -> Self {
        Self {
            slots: [0; M],
            entries: [(K::default(), V::default()); N],
        }
    }

    /// clears the slots bit map
    /// the map is effectively empty as the bitmap is used for occupancy queries
    #[inline]
    pub fn clear(&mut self) {
        self.slots.fill(0);
    }

    /// Returns the value of the key if it is present
    #[inline]
    pub fn get(&self, k: K) -> Option<V> 
    where K : Clone + Default + Hash + Eq
    {
        let start = Self::get_hash_idx(k);

        (start..N)
            .chain(0..start)
            .find(|&i| self.is_occupied(i) && self.entries[i].0 == k)
            .map(|i| self.entries[i].1)
    }

    /// adds an element to the map
    /// if the key is present the value is overwritten
    #[inline]
    pub fn put(&mut self, k: K, v: V) -> Option<usize> 
    where 
        K: Copy + Default + Hash + Eq,
        V: Copy + Default,
    {
        let start = Self::get_hash_idx(k);
        let probe = (start..N).chain(0..start);

        let idx = probe
            .clone()
            .find(|&i| self.entries[i].0 == k && self.is_occupied(i))
            .or_else(|| probe.clone().find(|&i| !self.is_occupied(i)))?;

        self.entries[idx] = (k,v);
        self.set_occupied(idx);

        Some(idx)
    }

    /// removes the mapping k,v if k is present in the map
    #[inline]
    pub fn remove(&mut self, k : K) -> Option<V> {
        let start = Self::get_hash_idx(k);
        let pos = (start..N)
            .chain(0..start)
            .find(|&i| self.is_occupied(i) && self.entries[i].0 == k)?;
       
        self.set_vacant(pos);

        Some(self.entries[pos].1)
    }

    /// returns an iterator of occupied tuples
    pub fn iter(&self) -> impl Iterator<Item=(K,V)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(i,_)| { self.is_occupied(*i) })
            .map(|(_,e)| *e)
    }
}

impl <K,V, const N : usize, const M : usize> Iterator for SmolMap<K,V,N,M> 
where 
    K : Copy + Default + Hash + Eq,
    V : Copy + Default
{
    type Item = (K,V);
    
    fn next(&mut self) -> Option<Self::Item> {
        self.entries.iter().next().cloned()
    }
}

impl <K,V, const N : usize, const M : usize> From<&[(K,V)]> for SmolMap<K,V,N,M> 
where 
    K : Copy + Default + Hash + Eq,
    V : Copy + Default
{
    fn from(values: &[(K,V)]) -> Self {
        let mut map = Self::new();
        for val in values {
            map.put(val.0, val.1);
        }

        map
    }
}

#[cfg(test)]
mod test {
    use crate::SmolMap;

    #[test]
    fn build_map() {
        let map = smol_map!(i32,u32,10);
        assert_eq!(map.len(),10);
    }

    #[test]
    fn test_put_one_element() {
        let mut map = smol_map!(i32,i32,1);
        
        map.put(10, 10);  
        assert_eq!(map.get(10),Some(10));

        map.put(10, 1);
        assert_eq!(map.get(10),Some(1));
    }

    #[test]
    fn get_one_element() {
        let mut map = smol_map!(u32,u32,1);
        map.put(10, 10);

        assert_eq!(map.get(10),Some(10));
    }

    #[test]
    fn get_element_not_present() {
        let mut map = smol_map!(i32,i32,2);
        map.put(10, -2);
        map.put(2, 3);

        assert_eq!(map.get(3),None);
    }

    #[test]
    fn test_iter() {
        let mut map = smol_map!(i32,i32,4);
        map.put(2,3);
        map.put(-1, 4);
        map.put(30, 5);

        assert_eq!(map.iter().count(), 3);
        assert!(map.iter().any(|(k,v)| k == 2 && v == 3));
        assert!(map.iter().any(|(k,v)| k == -1 && v == 4));
        assert!(map.iter().any(|(k,v)| k == 30 && v == 5));
    }

    // The entries in the array are initialized with default values of the types
    // Users can the add the default value to the map
    // A good test case would be to try and get the default value 
    // when the value is not explicitly added to the map 
    #[test]
    fn test_get_default_value(){
        let mut map = smol_map!(i32,i32,10);
        let val = map.get(0);

        // val should be none. Altough, the map is init with default value (default(i32) == 0)
        // get should yield None
        assert_eq!(val,None); 

        // explicitly add default value
        map.put(0,3);
        let val = map.get(0);
        
        assert_eq!(val,Some(3));
    }

    #[test]
    fn test_from_slice() {
        let slice = [(1,2),(2,3)].as_slice();
        let map = SmolMap::<i32,i32,10,1>::from(slice);

        assert_eq!(map.get(1),Some(2));
        assert_eq!(map.get(2),Some(3));
    }

    #[test]
    fn test_remove() {
        let mut map = smol_map!(i32,i32,10);

        for i in 1..=9 {
            map.put(i, i+10);
        }

        assert_eq!(map.get(1), Some(11));

        map.remove(1);

        assert_eq!(map.get(1), None);
    }

    #[test]
    fn test_remove_preserves_others() {
        let mut map = smol_map!(i32, i32, 10);
        for i in 1..=9 {
            map.put(i, i + 10);
        }

        assert_eq!(map.remove(1), Some(11));
        assert_eq!(map.get(1), None);
        // every other entry must survive the removal
        for i in 2..=9 {
            assert_eq!(map.get(i), Some(i + 10));
        }
        assert_eq!(map.iter().count(), 8);
    }

    #[test]
    fn test_get_after_remove_middle() {
        // Removing one key must not break lookup of keys
        // that probed past it at insert time.
        let mut map = smol_map!(i32, i32, 10);
        for i in 1..=9 {
            map.put(i, i * 100);
        }
        
        map.remove(5);
        assert_eq!(map.get(5), None);
        for i in [1, 2, 3, 4, 6, 7, 8, 9] {
            assert_eq!(map.get(i), Some(i * 100));
        }
    }

    #[test]
    fn test_put_after_remove_no_duplicate() {
        // Keys 1 and 6 share home slot 5, so 6 is stored at slot 6.
        // Removing 1 leaves a hole at 5; putting 6 again must
        // overwrite slot 6 rather than write a second copy into 5.
        let mut map = smol_map!(i32, i32, 10);
        for i in 1..=9 {
            map.put(i, i + 10);
        }

        map.remove(1);
        map.put(6, 99);
        assert_eq!(map.get(6), Some(99));
        assert_eq!(map.iter().filter(|e| e.0 == 6).count(), 1);

        map.remove(6);
        assert_eq!(map.get(6), None);
    }

    #[test]
    fn test_len_counts_occupied() {
        let mut map = smol_map!(i32, i32, 10);
        assert_eq!(map.iter().count(), 0);

        map.put(1, 10);
        map.put(2, 20);
        assert_eq!(map.iter().count(), 2);

        map.remove(1);
        assert_eq!(map.iter().count(), 1);
    }

    #[test]
    fn test_put_full_returns_none() {
        let mut map = smol_map!(i32, i32, 2);
        assert!(map.put(1, 10).is_some());
        assert!(map.put(2, 20).is_some());
        // map is full: a new key must fail without disturbing occupants
        assert_eq!(map.put(3, 30), None);
        assert_eq!(map.get(1), Some(10));
        assert_eq!(map.get(2), Some(20));
        assert_eq!(map.get(3), None);
    }

    #[test]
    fn test_overwrite_keeps_count() {
        let mut map = smol_map!(i32, i32, 4);
        map.put(1, 10);
        map.put(2, 20);
        assert_eq!(map.iter().count(), 2);

        map.put(1, 99);
        assert_eq!(map.get(1), Some(99));
        assert_eq!(map.iter().count(), 2);
    }

    #[test]
    fn test_clear() {
        let mut map = smol_map!(i32, i32, 10);
        for i in 1..=5 {
            map.put(i, i + 10);
        }
        assert_eq!(map.iter().count(), 5);

        map.clear();
        for i in 1..=5 {
            assert_eq!(map.get(i), None);
        }
        assert_eq!(map.iter().count(), 0);

        // map must be reusable after clear
        map.put(42, 42);
        assert_eq!(map.get(42), Some(42));
        assert_eq!(map.iter().count(), 1);
    }

    #[test]
    fn test_from_slice_truncates_to_capacity() {
        let vals: [(i32, i32); 4] = [(1, 10), (2, 20), (3, 30), (4, 40)];
        let map = SmolMap::<i32, i32, 2, 1>::from(&vals[..]);
        // must not panic; occupancy never exceeds capacity
        assert!(map.iter().count() <= 2);
    }

    #[test]
    fn test_multiword_bitmap() {
        // N = 70 needs M = 2 words; exercises the second bitmap word.
        let mut map = smol_map!(i32, i32, 70);
        for i in 0..70 {
            assert!(map.put(i, i + 1).is_some());
        }
        for i in 0..70 {
            assert_eq!(map.get(i), Some(i + 1));
        }
        assert_eq!(map.iter().count(), 70);

        assert_eq!(map.remove(0), Some(1));
        assert_eq!(map.remove(69), Some(70));
        assert_eq!(map.get(0), None);
        assert_eq!(map.get(69), None);
        assert_eq!(map.get(35), Some(36));
        assert_eq!(map.iter().count(), 68);
    }

    #[test]
    fn test_default_eq_new() {
        let a = SmolMap::<i32, i32, 4, 1>::new();
        let b = SmolMap::<i32, i32, 4, 1>::default();
        assert_eq!(a.iter().count(), 0);
        assert_eq!(b.iter().count(), 0);
        assert_eq!(a.get(1), None);
        assert_eq!(b.get(1), None);
    }

    #[test]
    fn test_iter_after_remove() {
        let mut map = smol_map!(i32, i32, 4);
        map.put(1, 10);
        map.put(2, 20);
        map.put(3, 30);
        map.remove(2);

        let mut seen = [false; 4];
        for (k, _) in map.iter() {
            seen[k as usize] = true;
        }
        assert!(seen[1]);
        assert!(!seen[2]);
        assert!(seen[3]);
        assert_eq!(map.iter().count(), 2);
    }
}