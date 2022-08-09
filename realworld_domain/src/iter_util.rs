use crate::error::*;

use anyhow::anyhow;

/// Iterator extension for extracting a single item
pub trait Single {
    type Item;

    /// Extract a single item from the iterator, erroring if there is less than or more than a single item in it.
    fn single(&mut self) -> RwResult<Self::Item>;

    /// Extract zero or one item from the iterator, erroring if there is more than a single item in it.
    fn single_or_none(&mut self) -> RwResult<Option<Self::Item>>;
}

impl<I: Iterator> Single for I {
    type Item = I::Item;

    fn single(&mut self) -> RwResult<Self::Item> {
        match (self.next(), self.next()) {
            (Some(item), None) => Ok(item),
            (None, _) => Err(anyhow!("Expected a single item, got none").into()),
            (Some(_), Some(_)) => Err(anyhow!("Expected a single itme, got more than one").into()),
        }
    }

    fn single_or_none(&mut self) -> RwResult<Option<Self::Item>> {
        match (self.next(), self.next()) {
            (None, None) => Ok(None),
            (Some(item), None) => Ok(Some(item)),
            (_, Some(_)) => Err(anyhow!("Expected a single itme, got more than one").into()),
        }
    }
}
