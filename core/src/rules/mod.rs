//! Правила: опечатки, «фокусы», частицы, ёфикатор, маскировка.

pub mod masking;
pub mod tricks;
pub mod typos;
pub mod yo;

pub use masking::{mask, MaskDirection, MaskPairs};
