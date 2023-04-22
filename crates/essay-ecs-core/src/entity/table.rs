use super::{meta::{TableId, TableType, ColumnId}, EntityId, column::RowId};

pub struct Table {
    _id: TableId,

    meta: TableType,

    rows: Vec<TableRow>,
}

#[derive(Debug)]
pub struct TableRow {
    id: RowId,

    columns: Vec<RowId>,
}

impl Table {
    pub(crate) fn new(id: TableId, meta: TableType) -> Self {
        Self {
            _id: id,
            meta,
            rows: Default::default(),
        }
    }

    pub(crate) fn meta(&self) -> &TableType {
        &self.meta
    }

    pub(crate) fn push(&mut self, columns: Vec<RowId>) -> RowId {
        let id = RowId::new(self.rows.len());

        let row = TableRow::new(id, columns);

        self.rows.push(row);

        id
    }

    pub(crate) fn get(&self, row_id: RowId) -> Option<&TableRow> {
        self.rows.get(row_id.index())
    }

    pub(crate) fn position(&self, column_id: ColumnId) -> Option<usize> {
        self.meta.position(column_id)
    }

    pub(crate) fn get_by_index(&self, row_index: usize) -> Option<&TableRow> {
        self.rows.get(row_index)
    }
}

impl TableRow {
    pub(crate) fn new(id: RowId, columns: Vec<RowId>) -> Self {
        Self {
            id,
            columns
        }
    }

    pub(crate) fn id(&self) -> RowId {
        self.id
    }

    pub(crate) fn column_row(&self, index: usize) -> RowId {
        self.columns[index]
    }

    pub(crate) fn column(&self, index: usize) -> RowId {
        self.columns[index]
    }
}
