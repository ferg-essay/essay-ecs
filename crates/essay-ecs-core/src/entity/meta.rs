use core::fmt;
use std::{
    collections::{HashMap}, 
    any::{TypeId, type_name}, 
    borrow::Cow, 
    alloc::Layout, 
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColumnId(usize);

#[derive(Debug,Clone,Copy,PartialEq,Hash,PartialOrd,Eq)]
pub struct TableId(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViewId(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViewTableId(usize);

#[derive(Clone)]
pub struct ColumnType {
    id: ColumnId,

    type_id: TypeId,
    name: Cow<'static, str>,

    layout: Layout,
    layout_padded: Layout,

    tables: Vec<TableId>,
    views: Vec<ViewId>,
}

pub struct TableType {
    id: TableId,

    columns: Vec<ColumnId>,
}

pub struct ViewType {
    id: ViewId,
    cols: Vec<ColumnId>,

    view_tables: Vec<ViewTableId>,
}

pub struct ViewTableType {
    id: ViewTableId,

    view_id: ViewId,
    table_id: TableId,

    index_map: Vec<usize>,
}

pub(crate) struct StoreMeta {
    column_map: HashMap<TypeId,ColumnId>,
    columns: Vec<ColumnType>,

    table_map: HashMap<Vec<ColumnId>,TableId>,
    tables: Vec<TableType>,

    view_map: HashMap<Vec<ColumnId>,ViewId>,
    views: Vec<ViewType>,

    view_table_map: HashMap<(ViewId,TableId), ViewTableId>,
    view_tables: Vec<ViewTableType>,
}

//
// implementation
//

//
// Column
//

impl ColumnId {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl ColumnType {
    pub fn id(&self) -> ColumnId {
        self.id
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.layout.size()
    }
    
    pub fn layout_padded(&self) -> &Layout {
        &self.layout_padded
    }

    #[inline]
    pub fn size_padded(&self) -> usize {
        self.layout_padded.size()
    }

    fn rows(&self) -> &Vec<TableId> {
        &self.tables
    }

    fn iter_rows(&self) -> impl Iterator<Item=&TableId> {
        self.tables.iter()
    }
}

impl fmt::Debug for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColumnType")
         .field("id", &self.id)
         .field("name", &self.name)
         .finish()
    }
}

//
// Table
//

impl TableId {
    pub const INVALID: TableId = TableId(usize::MAX);

    #[inline]
    pub fn index(&self) -> usize {
        self.0
    }
}

impl TableType {
    pub(crate) fn id(&self) -> TableId {
        self.id
    }

    pub(crate) fn columns(&self) -> &Vec<ColumnId> {
        &self.columns
    }

    fn contains_column(&self, column: ColumnId) -> bool {
        self.columns.iter().any(|col| *col == column)
    }

    pub fn find_column(&self, id: ColumnId) -> Option<&ColumnId> {
        self.columns.iter().find(|col| **col == id)
    }

    pub fn position(&self, id: ColumnId) -> Option<usize> {
        self.columns.iter().position(|col| *col == id)
    }

    fn contains_columns(&self, cols: &Vec<ColumnId>) -> bool {
        for col in cols {
            if self.find_column(*col).is_none() {
                return false;
            }
        }

        true
    }
}

impl fmt::Debug for TableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableType")
         .field("id", &self.id)
         .field("columns", &self.columns)
         .finish()
    }
}

//
// View
//

impl ViewId {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl ViewType {
    pub fn id(&self) -> ViewId {
        self.id
    }

    pub(crate) fn column_position(&self, col_id: ColumnId) -> Option<usize> {
        self.cols.iter().position(|col| *col == col_id)
    }

    pub(crate) fn view_tables(&self) -> &Vec<ViewTableId> {
        &self.view_tables
    }
}

impl fmt::Debug for ViewType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewType")
         .field("id", &self.id)
         .field("cols", &self.cols)
         .finish()
    }
}

//
// ViewRow
//

impl ViewTableId {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl ViewTableType {
    pub fn new(
        id: ViewTableId, 
        table: &TableType, 
        view: &ViewType
    ) -> ViewTableType {
        let mut index_map = Vec::<usize>::new();

        for col in &view.cols {
            let index = table.columns().iter()
                .position(|c| c == col).unwrap();

            index_map.push(index);
        }

        ViewTableType {
            id,
            view_id: view.id,
            table_id: table.id(),
            index_map,
        }
    }

    pub fn id(&self) -> ViewTableId {
        self.id
    }

    pub(crate) fn view_id(&self) -> ViewId {
        self.view_id
    }

    pub(crate) fn table_id(&self) -> TableId {
        self.table_id
    }

    pub fn index_map(&self) -> &Vec<usize> {
        &self.index_map
    }
}

impl fmt::Debug for ViewTableType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewTableType")
         .field("id", &self.id)
         .field("table_id", &self.table_id)
         .field("view_id", &self.view_id)
         .finish()
    }
}

//
// TableMeta
//

impl StoreMeta {
    pub fn new() -> Self {
        Self {
            column_map: HashMap::new(),
            columns: Vec::new(),

            table_map: HashMap::new(),
            tables: Vec::new(),

            view_map: HashMap::new(),
            views: Vec::new(),

            view_table_map: HashMap::new(),
            view_tables: Vec::new(),
        }
    }

    //
    // Column
    //

    pub fn column(&self, id: ColumnId) -> &ColumnType {
        self.columns.get(id.index()).unwrap()
    }

    pub fn column_mut(&mut self, id: ColumnId) -> &mut ColumnType {
        self.columns.get_mut(id.index()).unwrap()
    }

    pub(crate) fn get_column<T:'static>(&self) -> Option<ColumnId> {
        match self.column_map.get(&TypeId::of::<T>()) {
            Some(column_type_id) => {
                Some(ColumnId(column_type_id.index()))
            },
            None => None,
        }
    }

    pub fn add_column<T:'static>(&mut self) -> ColumnId {
        let type_id = TypeId::of::<T>();

        let id = *self.column_map.entry(type_id)
            .or_insert(ColumnId(self.columns.len()));

        if self.columns.len() == id.index() {
            let col_type = ColumnType {
                id: id,

                type_id: TypeId::of::<T>(),
                name: Cow::Borrowed(type_name::<T>()),

                layout: Layout::new::<T>(),
                layout_padded: Layout::new::<T>().pad_to_align(),

                tables: Vec::new(),
                views: Vec::new(),
            };

            self.columns.push(col_type);
        }

        id
    }

    //
    // Table
    //

    pub fn table(&self, id: TableId) -> &TableType {
        &self.tables[id.index()]
    }

    pub(crate) fn table_mut(&mut self, id: TableId) -> &mut TableType {
        self.tables.get_mut(id.index()).unwrap()
    }

    pub fn add_table(&mut self, mut columns: Vec<ColumnId>) -> TableId {
        columns.sort();
        columns.dedup();

        let len = self.tables.len();
        let table_id = *self.table_map.entry(columns.clone()).or_insert_with(|| {
            TableId(len)
        });

        if table_id.index() < len {
            return table_id;
        }

        self.tables.push(TableType {
            id: table_id,
            columns,
        });

        self.fill_table_columns(table_id);
        self.add_view_tables_from_table(table_id);

        table_id
    }

    fn fill_table_columns(&mut self, table_id: TableId) {
        let columns = self.table(table_id).columns().clone();

        for column_id in columns {
            let column = self.column_mut(column_id);

            column.tables.push(table_id);
        }
    }

    //
    // View
    //

    pub fn view(&self, id: ViewId) -> &ViewType {
        self.views.get(id.index()).unwrap()
    }

    fn view_mut(&mut self, id: ViewId) -> &mut ViewType {
        self.views.get_mut(id.index()).unwrap()
    }

    pub(crate) fn get_view(&self, cols: &Vec<ColumnId>) -> Option<ViewId> {
        match self.view_map.get(cols) {
            Some(type_id) => Some(*type_id),
            None => None,
        }
    }

    pub(crate) fn add_view(&mut self, columns: &Vec<ColumnId>) -> ViewId {
        let len = self.views.len();

        let view_id = *self.view_map
            .entry(columns.clone())
            .or_insert_with(|| {
            ViewId(len)
        });

        if view_id.0 == len {
            self.views.push(ViewType {
                id: view_id,
                cols: columns.clone(),
                view_tables: Vec::new(),
            });

            self.add_view_tables_from_view(view_id);
        }

        view_id
    }

    //
    // ViewTable
    //

    pub fn view_table(&self, id: ViewTableId) -> &ViewTableType {
        self.view_tables.get(id.index()).unwrap()
    }

    pub(crate) fn add_view_table(
        &mut self,
        table_id: TableId, 
        view_id: ViewId
    ) -> ViewTableId {
        let len = self.view_tables.len();

        let view_table_id = *self.view_table_map
            .entry((view_id, table_id))
            .or_insert_with(|| {
            ViewTableId(len)
        });

        if view_table_id.index() == len {
            self.push_view_table(table_id, view_id, view_table_id);
        }

        view_table_id
    }

    fn push_view_table(
        &mut self, 
        table_id: TableId,
        view_id: ViewId, 
        view_table_id: ViewTableId
    ) {
        let table = self.table(table_id);
        let view_type = self.view(view_id);

        assert_eq!(view_table_id.index(), self.view_tables.len());

        self.view_tables.push(ViewTableType::new(view_table_id, table, view_type));

        let view_type = self.view_mut(view_id);
        view_type.view_tables.push(view_table_id);
    }

    fn add_view_tables_from_table(
        &mut self, 
        table_id: TableId, 
    ) {
        let table = self.table(table_id);

        let mut views: Vec<ViewId> = Vec::new();

        for view_type in &self.views {
            if table.contains_columns(&view_type.cols) {
                views.push(view_type.id());
            }
        }

        for view_id in views {
            self.add_view_table(table_id, view_id);
        }
    }

    fn add_view_tables_from_view(&mut self, view_id: ViewId) {
        let view_type = self.view(view_id);
        let cols = view_type.cols.clone();

        for col in &cols {
            let col_type = self.column_mut(*col);

            col_type.views.push(view_id);
        }

        let mut match_tables = Vec::<TableId>::new();

        for table in &self.tables {
            if table.contains_columns(&cols) {
                match_tables.push(table.id());
            }
        }

        for table_id in match_tables {
            self.add_view_table(table_id, view_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{mem, alloc::Layout};

    use crate::entity::meta::{ColumnId, ViewId, ViewTableId, TableId};

    use super::StoreMeta;

    #[test]
    fn add_column() {
        let mut meta = StoreMeta::new();

        let col_id = meta.add_column::<TestA>();
        let col_type = meta.column(col_id);
        assert_eq!(col_type.id(), ColumnId(0));
        assert_eq!(col_type.size(), mem::size_of::<usize>());
        assert_eq!(col_type.size_padded(), mem::size_of::<usize>());
        //assert_eq!(col_type.layout(), &Layout::new::<TestA>());
        assert_eq!(col_type.layout_padded(), &Layout::new::<TestA>().pad_to_align());
        assert_eq!(col_type.tables.len(), 0);
        assert_eq!(col_type.views.len(), 0);

        let col_id = meta.add_column::<TestB>();
        let col_type = meta.column(col_id);
        assert_eq!(col_type.id(), ColumnId(1));
        assert_eq!(col_type.size(), mem::size_of::<usize>());
        //assert_eq!(col_type.align(), mem::align_of::<usize>());
        assert_eq!(col_type.tables.len(), 0);
        assert_eq!(col_type.views.len(), 0);

        // check double add
        let col_id = meta.add_column::<TestA>();
        assert_eq!(col_id, ColumnId(0));
    }

    #[test]
    fn add_single_row() {
        let mut meta = StoreMeta::new();

        let type_a_id = single_row_type::<TestA>(&mut meta);
        assert_eq!(type_a_id, TableId(0));

        let type_a = meta.table(type_a_id);
        assert_eq!(type_a.id(), TableId(0));
        let cols = type_a.columns();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], ColumnId(0));

        let col_a = meta.column(ColumnId(0));
        assert_eq!(col_a.tables.len(), 1);
        assert_eq!(col_a.tables[0], TableId(0));
        assert_eq!(col_a.views.len(), 0);

        let type_a_id = single_row_type::<TestA>(&mut meta);
        assert_eq!(type_a_id, TableId(0));

        let type_a = meta.table(type_a_id);
        assert_eq!(type_a.id(), TableId(0));

        let cols = type_a.columns();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], ColumnId(0));

        let col_a = meta.column(ColumnId(0));
        assert_eq!(col_a.tables.len(), 1);
        assert_eq!(col_a.views.len(), 0);

        let type_b_id = single_row_type::<TestB>(&mut meta);
        assert_eq!(type_b_id, TableId(1));

        let type_b = meta.table(type_b_id);
        assert_eq!(type_b.id(), TableId(1));
        let cols = type_b.columns();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], ColumnId(1));

        let col_b = meta.column(ColumnId(1));
        assert_eq!(col_b.tables.len(), 1);
        assert_eq!(col_b.tables[0], TableId(1));
        assert_eq!(col_b.views.len(), 0);
    }

    fn single_row_type<T:'static>(meta: &mut StoreMeta) -> TableId {
        let column_id = meta.add_column::<T>();
        let mut columns = Vec::<ColumnId>::new();
        columns.push(column_id);

        meta.add_table(columns)
    }

    #[test]
    fn push_row() {
        let mut meta = StoreMeta::new();

        let type_a_id = single_row_type::<TestA>(&mut meta);
        assert_eq!(type_a_id, TableId(0));

        let type_a = meta.table(type_a_id);
        assert_eq!(type_a.id(), TableId(0));

        /*
        let type_aa_id = meta.push_row_by_type::<TestA>(type_a_id);
        assert_eq!(type_aa_id, RowTypeId(0));

        let type_aa = meta.get_row(type_aa_id);
        assert_eq!(type_aa.id(), RowTypeId(0));

        let cols = type_aa.columns();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], ColumnId(0));

        let type_b_id = meta.single_row_type::<TestB>();
        assert_eq!(type_b_id, RowTypeId(1));

        let type_ab_id = meta.push_row_by_type::<TestB>(type_a_id);
        assert_eq!(type_ab_id, RowTypeId(2));

        let type_ab = meta.get_row(type_ab_id);
        assert_eq!(type_ab.id(), RowTypeId(2));

        let cols = type_ab.columns();
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0], ColumnId(0));
        assert_eq!(cols[1], ColumnId(1));

        let col_a = meta.get_column(ColumnId(0));
        assert_eq!(col_a.rows.len(), 2);
        assert_eq!(col_a.rows[0], RowTypeId(0));
        assert_eq!(col_a.rows[1], RowTypeId(2));

        let col_b = meta.get_column(ColumnId(1));
        assert_eq!(col_b.rows.len(), 2);
        assert_eq!(col_b.rows[0], RowTypeId(1));
        assert_eq!(col_b.rows[1], RowTypeId(2));

        let type_aba = meta.push_row_by_type::<TestA>(type_ab_id);
        assert_eq!(type_aba, RowTypeId(2));

        let type_ba = meta.push_row_by_type::<TestA>(type_b_id);
        assert_eq!(type_ba, RowTypeId(2));
        */
    }

    #[test]
    fn row_cols() {
        let mut meta = StoreMeta::new();

        let type_a = single_row_type::<TestA>(&mut meta);
        assert_eq!(type_a, TableId(0));

        let col_a = meta.add_column::<TestA>();

        let row_type = meta.table(type_a);
        assert_eq!(row_type.id(), type_a);
        assert_eq!(row_type.columns().len(), 1);
        let cols = row_type.columns();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], col_a);

        let type_b = single_row_type::<TestB>(&mut meta);
        assert_eq!(type_b, TableId(1));

        let col_b = meta.add_column::<TestB>();

        /*
        let type_ba = meta.push_row_by_type::<TestA>(type_b);
        assert_eq!(type_ba, RowTypeId(2));

        let row_type = meta.get_row(type_ba);
        assert_eq!(row_type.id(), type_ba);
        assert_eq!(row_type.columns().len(), 2);
        let cols = row_type.columns();
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0], col_a);
        assert_eq!(cols[1], col_b);
        */
    }

    #[test]
    fn col_rows() {
        let mut meta = StoreMeta::new();

        let type_a = single_row_type::<TestA>(&mut meta);
        assert_eq!(type_a, TableId(0));

        /*
        let col_a = meta.add_column::<TestA>();
        let rows: Vec<RowTypeId> = meta
            .select_rows_by_column(col_a)
            .map(|id| *id).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], RowTypeId(0));

        let col_c = meta.add_column::<TestC>();
        let col_b = meta.add_column::<TestB>();

        let type_c = meta.single_row_type::<TestC>();
        assert_eq!(type_c, RowTypeId(1));
        */

        /*
        let type_cb = meta.push_row_by_type::<TestB>(type_c);
        assert_eq!(type_cb, RowTypeId(2));

        let type_cba = meta.push_row_by_type::<TestA>(type_cb);
        assert_eq!(type_cba, RowTypeId(3));

        let rows: Vec<RowTypeId> = meta.col_rows(col_a).map(|id| *id).collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], RowTypeId(0));
        assert_eq!(rows[1], RowTypeId(3));

        let rows: Vec<RowTypeId> = meta.col_rows(col_b).map(|id| *id).collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], RowTypeId(2));
        assert_eq!(rows[1], RowTypeId(3));

        let rows: Vec<RowTypeId> = meta.col_rows(col_c).map(|id| *id).collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], RowTypeId(1));
        assert_eq!(rows[1], RowTypeId(2));
        assert_eq!(rows[2], RowTypeId(3));
        */
    }

    #[test]
    fn row_then_view() {
        let mut meta = StoreMeta::new();

        let row_id_a = single_row_type::<TestA>(&mut meta);
        assert_eq!(row_id_a, TableId(0));

        let view_id_a = add_view_single::<TestA>(&mut meta);
        assert_eq!(view_id_a, ViewId(0));

        let view_a = meta.view(view_id_a);
        assert_eq!(view_a.id(), ViewId(0));
        let cols = &view_a.cols;
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], ColumnId(0));

        let col_a = meta.column(ColumnId(0));
        assert_eq!(col_a.tables.len(), 1);
        assert_eq!(col_a.tables[0], TableId(0));
        assert_eq!(col_a.views.len(), 1);
        assert_eq!(col_a.views[0], ViewId(0));

        let entity_row_a = meta.view_table(ViewTableId(0));
        assert_eq!(entity_row_a.id(), ViewTableId(0));
        assert_eq!(entity_row_a.table_id(), TableId(0));
        assert_eq!(entity_row_a.view_id(), ViewId(0));
    }

    #[test]
    fn view_then_row() {
        let mut meta = StoreMeta::new();

        let entity_id_a = add_view_single::<TestA>(&mut meta);
        assert_eq!(entity_id_a, ViewId(0));

        let row_id_a = single_row_type::<TestA>(&mut meta);
        assert_eq!(row_id_a, TableId(0));

        //meta.push_row(row_id_a, col_id_b);

        let entity_a = meta.view(entity_id_a);
        assert_eq!(entity_a.id(), ViewId(0));
        let cols = &entity_a.cols;
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], ColumnId(0));
        let rows = &entity_a.view_tables;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], ViewTableId(0));

        let col_a = meta.column(ColumnId(0));
        assert_eq!(col_a.tables.len(), 1);
        assert_eq!(col_a.tables[0], TableId(0));
        assert_eq!(col_a.views.len(), 1);
        assert_eq!(col_a.views[0], ViewId(0));

        let entity_row_a = meta.view_table(ViewTableId(0));
        assert_eq!(entity_row_a.id(), ViewTableId(0));
        assert_eq!(entity_row_a.table_id(), TableId(0));
        assert_eq!(entity_row_a.view_id(), ViewId(0));
    }

    #[test]
    fn view_then_row2() {
        let mut meta = StoreMeta::new();

        let entity_id_a = add_view_single::<TestA>(&mut meta);
        assert_eq!(entity_id_a, ViewId(0));

        let entity_id_b = add_view_single::<TestB>(&mut meta);
        assert_eq!(entity_id_b, ViewId(1));

        let row_id_a = single_row_type::<TestA>(&mut meta);
        assert_eq!(row_id_a, TableId(0));

        let row_id_b = single_row_type::<TestB>(&mut meta);
        assert_eq!(row_id_b, TableId(1));

        let entity_b = meta.view(entity_id_b);
        assert_eq!(entity_b.id(), ViewId(1));
        let cols = &entity_b.cols;
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], ColumnId(1));
        let rows = &entity_b.view_tables;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], ViewTableId(1));

        let col_b = meta.column(ColumnId(1));
        assert_eq!(col_b.tables.len(), 1);
        assert_eq!(col_b.tables[0], TableId(1));
        assert_eq!(col_b.views.len(), 1);
        assert_eq!(col_b.views[0], ViewId(1));

        let entity_row_b = meta.view_table(ViewTableId(1));
        assert_eq!(entity_row_b.id(), ViewTableId(1));
        assert_eq!(entity_row_b.table_id(), TableId(1));
        assert_eq!(entity_row_b.view_id(), ViewId(1));
    }

    fn add_view_single<T:'static>(meta: &mut StoreMeta) -> ViewId {
        let column_id = meta.add_column::<T>();
        let mut columns = Vec::<ColumnId>::new();
        columns.push(column_id);

        meta.add_view(&columns)
    }

    struct TestA(usize);
    struct TestB(usize);
    struct TestC(usize);
}