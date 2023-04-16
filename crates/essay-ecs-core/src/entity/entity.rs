use super::{meta::TableId, column::RowId};


#[derive(Debug,Clone,Copy,PartialEq,Hash,PartialOrd,Eq)]
pub struct EntityId(usize);

#[derive(Debug)]
pub struct Entity {
    id: EntityId,
    table: TableId,

    columns: Vec<RowId>,
}

impl EntityId {
    pub(crate) fn index(&self) -> usize {
        self.0
    }
}

impl Entity {
    pub(crate) fn new(len: usize, table: TableId, columns: Vec<RowId>) -> Self {
        Self {
            id: EntityId(len),
            table,
            columns
        }
    }

    pub(crate) fn id(&self) -> EntityId {
        self.id
    }

    pub(crate) fn table(&self) -> TableId {
        self.table
    }

    pub(crate) fn column_row(&self, index: usize) -> RowId {
        self.columns[index]
    }

    pub(crate) fn column(&self, index: usize) -> RowId {
        self.columns[index]
    }
}
