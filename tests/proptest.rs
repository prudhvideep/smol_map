use proptest::prelude::*;
use smol_map::smol_map;
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum Op {
    Put(i32, i32),
    Get(i32),
    Remove(i32),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..15i32).prop_map(Op::Get),
        (0..15i32).prop_map(Op::Remove),
        (0..15i32, any::<i32>()).prop_map(|(k, v)| Op::Put(k, v))
    ]
}

proptest! {
    #[test]
    fn behaves_like_hashmap(ops in prop::collection::vec(op(), 0..200)) {
        let mut map = smol_map!(i32, i32, 10);
        let mut model = HashMap::new();

        for op in ops {
            match op {
                Op::Put(k, v) => {
                    if map.put(k, v).is_some() {
                        model.insert(k, v);
                    }
                }
                Op::Remove(k) => prop_assert_eq!(map.remove(k), model.remove(&k)),
                Op::Get(k) => prop_assert_eq!(map.get(k), model.get(&k).copied()),
            }
            prop_assert_eq!(map.iter().count(), model.len());
        }
    }
}
