use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use hashmemo::HashMemo;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    hash::{BuildHasher, Hash},
};

use ahash::RandomState as AHashBuilder;

#[derive(Clone, Eq, PartialEq, Hash)]
struct BigStruct {
    name: String,
    data: [u64; 64],
    payload: Vec<u8>,
}

impl BigStruct {
    fn new(name: String) -> Self {
        Self {
            name,
            data: [42; 64],
            payload: vec![7; 1024],
        }
    }
}

#[inline]
fn move_things_around<T: Eq + Hash, S: BuildHasher>(
    map1: &mut HashMap<T, (), S>,
    map2: &mut HashMap<T, (), S>,
    steps: usize,
) {
    for _ in 0..steps {
        for (k, v) in map1.drain() {
            map2.insert(k, v);
        }
        for (k, v) in map2.drain() {
            map1.insert(k, v);
        }
    }
}

struct Param {
    map_size: usize,
    word_length: usize,
    steps: usize,
    variant: &'static str,
}

impl Display for Param {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} elems | word_len={} | steps={} | {}",
            self.map_size, self.word_length, self.steps, self.variant
        )
    }
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("HashMemo vs AHash");

    for &map_size in [100, 1000].iter() {
        for &word_length in [10, 100].iter() {
            for &steps in [5].iter() {
                // --- Data: small string keys ---
                let string_keys: Vec<_> = (0..map_size)
                    .map(|i| i.to_string().repeat(word_length))
                    .collect();

                // DefaultHasher
                bench_hashmap::<_, std::collections::hash_map::RandomState>(
                    &mut group,
                    "String",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "DefaultHasher",
                    },
                    &string_keys,
                );

                bench_hashmap::<HashMemo<String>, std::collections::hash_map::RandomState>(
                    &mut group,
                    "HashMemo<String>",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "DefaultHasher",
                    },
                    &string_keys.iter().cloned().map(HashMemo::new).collect(),
                );

                // AHash
                bench_hashmap::<_, AHashBuilder>(
                    &mut group,
                    "String",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "AHash",
                    },
                    &string_keys,
                );

                bench_hashmap::<HashMemo<String>, AHashBuilder>(
                    &mut group,
                    "HashMemo<String>",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "AHash",
                    },
                    &string_keys.iter().cloned().map(HashMemo::new).collect(),
                );

                // --- Data: big struct ---
                let bigs: Vec<_> = (0..map_size)
                    .map(|i| BigStruct::new(i.to_string().repeat(word_length)))
                    .collect();

                bench_hashmap::<_, std::collections::hash_map::RandomState>(
                    &mut group,
                    "BigStruct",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "DefaultHasher",
                    },
                    &bigs,
                );

                bench_hashmap::<HashMemo<BigStruct>, std::collections::hash_map::RandomState>(
                    &mut group,
                    "HashMemo<BigStruct>",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "DefaultHasher",
                    },
                    &bigs.iter().cloned().map(HashMemo::new).collect(),
                );

                bench_hashmap::<_, AHashBuilder>(
                    &mut group,
                    "BigStruct",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "AHash",
                    },
                    &bigs,
                );

                bench_hashmap::<HashMemo<BigStruct>, AHashBuilder>(
                    &mut group,
                    "HashMemo<BigStruct>",
                    Param {
                        map_size,
                        word_length,
                        steps,
                        variant: "AHash",
                    },
                    &bigs.iter().cloned().map(HashMemo::new).collect(),
                );
            }
        }
    }

    group.finish();
}

fn bench_hashmap<T, S>(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    param: Param,
    data: &Vec<T>,
) where
    T: Eq + Hash + Clone,
    S: BuildHasher + Default,
{
    group.bench_with_input(BenchmarkId::new(name, &param), data, |b, data| {
        b.iter(|| {
            let mut map: HashMap<T, (), S> = HashMap::with_hasher(S::default());
            for key in data.iter().cloned() {
                map.insert(key, ());
            }
            move_things_around(&mut map, &mut HashMap::with_hasher(S::default()), param.steps);
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);