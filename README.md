# SmolMap

A stack allocated map similar to llvm `DenseMap`. The keys and values live in a single allocation and the collisions are resolved using linear probing.

![Animated SmolMap layout: three puts into a 10-slot map, with key 6 probing past slot 5](docs/smol_map.svg)

