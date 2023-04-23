use super::{meta::{TableId, TableMeta, ColumnId}, column::RowId};

pub struct Table {
    _id: TableId,

    meta: TableMeta,

    rows: Vec<TableRow>,

    free_list: Vec<RowId>,
}

#[derive(Debug)]
pub struct TableRow {
    id: RowId,

    columns: Vec<RowId>,
}

impl Table {
    pub(crate) fn new(id: TableId, meta: TableMeta) -> Self {
        Self {
            _id: id,
            meta,
            rows: Default::default(),
            free_list: Default::default(),
        }
    }

    pub(crate) fn meta(&self) -> &TableMeta {
        &self.meta
    }

    pub(crate) fn position(&self, column_id: ColumnId) -> Option<usize> {
        self.meta.position(column_id)
    }

    pub(crate) fn get(&self, row_id: RowId) -> Option<&TableRow> {
        let row = &self.rows[row_id.index()];

        if row.id == row_id {
            Some(row)
        } else {
            None
        }
    }

    pub(crate) fn get_by_index(&self, row_index: usize) -> Option<&TableRow> {
        self.rows.get(row_index)
    }

    pub(crate) fn push(&mut self, columns: Vec<RowId>) -> RowId {
        if let Some(id) = self.free_list.pop() {
            self.rows[id.index()] = TableRow::new(id.allocate(), columns);

            id
        } else {
            let id = RowId::new(self.rows.len());

            let row = TableRow::new(id, columns);

            self.rows.push(row);

            id
        }
    }

    pub(crate) fn remove(&mut self, row_id: RowId) {
        let row = &mut self.rows[row_id.index()];

        if row.id == row_id {
            row.id = row_id.next_free();
            self.free_list.push(row.id);
        }
    }
}

impl TableRow {
    pub(crate) fn new(id: RowId, columns: Vec<RowId>) -> Self {
        Self {
            id,
            columns
        }
    }

    #[inline]
    pub(crate) fn is_alloc(&self) -> bool {
        self.id.is_alloc()
    }

    #[inline]
    pub(crate) fn column_row(&self, index: usize) -> RowId {
        self.columns[index]
    }

    #[inline]
    pub(crate) fn column(&self, index: usize) -> RowId {
        self.columns[index]
    }

    #[inline]
    pub(crate) fn columns(&self) -> &Vec<RowId> {
        &self.columns
    }
}
