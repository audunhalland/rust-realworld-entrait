use crate::error::*;

use anyhow::anyhow;

/// Iterator extension for extracting a single item
pub trait Single {
    type Item;

    fn single(&mut self) -> RwResult<Self::Item>;
    fn single_or_none(&mut self) -> RwResult<Option<Self::Item>>;
}

impl<I> Single for I
where
    I: Iterator,
{
    type Item = I::Item;

    fn single(&mut self) -> RwResult<Self::Item> {
        match self.next() {
            None => Err(anyhow!("Expected a single item, got none").into()),
            Some(item) => match self.next() {
                Some(_) => Err(anyhow!("Expected a single itme, get more than one").into()),
                None => Ok(item),
            },
        }
    }

    fn single_or_none(&mut self) -> RwResult<Option<Self::Item>> {
        match self.next() {
            None => Ok(None),
            Some(item) => match self.next() {
                Some(_) => Err(anyhow!("Expected a single itme, get more than one").into()),
                None => Ok(Some(item)),
            },
        }
    }
}
