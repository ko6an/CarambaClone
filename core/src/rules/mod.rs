//! Правила: опечатки, «фокусы», частицы, ёфикатор, маскировка.

pub mod masking;
pub mod pack;
pub mod tricks;
pub mod typos;
pub mod yo;

pub use masking::{mask, MaskDirection, MaskPairs};
pub use pack::{PackPayload, SignedPack};
