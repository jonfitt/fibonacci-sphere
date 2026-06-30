//! Object-safe random helpers for terrain generation.

use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::StdRng;

use super::types::TerrainType;

/// Build an RNG for terrain generation from an optional fixed seed.
pub fn terrain_rng_from_seed(seed: Option<u32>) -> StdRng {
    let seed = seed.unwrap_or_else(rand::random);
    StdRng::seed_from_u64(u64::from(seed))
}

/// Build an RNG from a Godot-style seed (`seed < 0` picks a random seed).
pub fn terrain_rng_from_godot_seed(seed: i32) -> StdRng {
    terrain_rng_from_seed(if seed < 0 { None } else { Some(seed as u32) })
}

pub(crate) fn random_index(rng: &mut dyn RngCore, len: usize) -> usize {
    debug_assert!(len > 0);
    (rng.next_u32() as usize) % len
}

pub(crate) fn random_terrain(rng: &mut dyn RngCore) -> TerrainType {
    TerrainType::ALL[random_index(rng, TerrainType::ALL.len())]
}

pub(crate) fn choose<T: Copy>(items: &[T], rng: &mut dyn RngCore) -> T {
    items[random_index(rng, items.len())]
}

#[allow(dead_code)]
pub(crate) fn shuffle<T>(rng: &mut dyn RngCore, items: &mut [T]) {
    if items.len() < 2 {
        return;
    }
    for index in (1..items.len()).rev() {
        let swap_index = random_index(rng, index + 1);
        items.swap(index, swap_index);
    }
}

#[allow(dead_code)]
pub(crate) fn shuffle_slice(rng: &mut dyn RngCore, items: &mut [usize]) {
    shuffle(rng, items);
}
