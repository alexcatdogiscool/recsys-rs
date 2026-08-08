
pub use crate::Item;

#[derive(Clone, Debug, Default)]
pub struct Profile {
    pub engage_history: Vec<Item>,
    pub ignore_history: Vec<Item>,
}