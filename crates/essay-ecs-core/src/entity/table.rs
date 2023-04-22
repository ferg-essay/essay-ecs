use super::{meta::TableId, EntityId};

pub struct Table {
    _id: TableId,

    entities: Vec<EntityId>,
}

impl Table {
    pub(crate) fn new(id: TableId) -> Self {
        Self {
            _id: id,
            entities: Default::default(),
        }
    }

    pub(crate) fn push(&mut self, id: EntityId) {
        self.entities.push(id)
    }

    pub(crate) fn get(&self, row_index: usize) -> Option<&EntityId> {
        self.entities.get(row_index)
    }
}