use super::{meta::{TableId, TableMeta, ColumnId}, column::RowId, EntityId};

pub struct Table {
    _id: TableId,

    meta: TableMeta,

    rows: Vec<TableRow>,

    free_list: Vec<RowId>,
}

#[derive(Debug)]
pub struct TableRow {
    entity_id: EntityId,
    row_id: RowId,

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

        if row.row_id == row_id {
            Some(row)
        } else {
            None
        }
    }

    pub(crate) fn get_by_index(&self, row_index: usize) -> Option<&TableRow> {
        self.rows.get(row_index)
    }

    pub(crate) fn push(
        &mut self, 
        entity_id: EntityId, 
        columns: Vec<RowId>
    ) -> RowId {
        if let Some(row_id) = self.free_list.pop() {
            self.rows[row_id.index()] = TableRow::new(
                entity_id, 
                row_id.allocate(), 
                columns
            );

            row_id
        } else {
            let row_id = RowId::new(self.rows.len());

            let row = TableRow::new(entity_id, row_id, columns);

            self.rows.push(row);

            row_id
        }
    }

    pub(crate) fn remove(&mut self, row_id: RowId) {
        let row = &mut self.rows[row_id.index()];

        if row.row_id == row_id {
            row.row_id = row_id.next_free();
            self.free_list.push(row.row_id);
        }
    }
}

impl TableRow {
    pub(crate) fn new(
        entity_id: EntityId,
        row_id: RowId, 
        columns: Vec<RowId>
    ) -> Self {
        Self {
            entity_id,
            row_id,
            columns
        }
    }

    #[inline]
    pub(crate) fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    #[inline]
    pub(crate) fn is_alloc(&self) -> bool {
        self.row_id.is_alloc()
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
