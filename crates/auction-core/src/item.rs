use crate::types::{ItemId, Money};

#[derive(Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub reserve_price: Option<Money>,
}
