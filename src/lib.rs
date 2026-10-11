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
    #[inline]
    fn is_occupied(&self, i: usize) -> bool {
        ((self.slots[i / 64] >> (i % 64)) & 1) == 1
    }

    #[inline]
    fn set_occupied(&mut self, i: usize) {
        self.slots[i / 64] |= 1 << (i % 64);
    }

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
            .take_while(|&i| self.is_occupied(i))
            .find(|&i| self.entries[i].0 == k)
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
        let pos = (start..N)
            .chain(0..start)
            .find(|&i| !self.is_occupied(i) || self.entries[i].0 == k)?;

        self.entries[pos] = (k,v);
        self.set_occupied(pos);

        Some(pos)
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
}