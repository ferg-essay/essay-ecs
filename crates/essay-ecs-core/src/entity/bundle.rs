
//
// Insert
//

use super::{meta::{TableId, ColumnId, TableMeta}, Store, column::RowId, Component, store::EntityId};

pub trait Bundle:'static {
    fn build(builder: &mut InsertBuilder);

    unsafe fn insert(cursor: &mut InsertCursor, value: Self);
}

pub struct InsertBuilder<'a> {
    store: &'a mut Store,
    columns: Vec<ColumnId>,
}

pub struct InsertPlan {
    table_id: TableId,
    columns: Vec<ColumnId>,
    index_map: Vec<usize>,
}

pub struct InsertCursor<'a> {
    id: EntityId,
    store: &'a mut Store,
    plan: &'a InsertPlan,
    index: usize,
    rows: Vec<RowId>,
}

impl<'a,'t> InsertBuilder<'a> {
    pub(crate) fn new(store: &'a mut Store) -> Self {
        Self {
            store,
            columns: Vec::new(),
        }
    }

    pub(crate) fn add_entity(&mut self, id: EntityId) {
        for col_id in self.store.entity_column_ids(id) {
            self.columns.push(*col_id);
        }
    }

    pub(crate) fn _add_table(&mut self, table: &TableMeta) {
        for col_id in table.columns() {
            self.columns.push(*col_id);
        }
    }

    pub(crate) fn add_column<T:'static>(&mut self) {
        let id = self.store.add_column::<T>();
        
        self.columns.push(id);
    }

    pub(crate) fn build(self) -> InsertPlan {
        let table_id = self.store.add_table(self.columns.clone());
        let table = self.store.meta().table(table_id);

        let mut index_map = Vec::<usize>::new();

        for table_column in table.columns() {
            index_map.push(self.columns.iter()
                .position(|c| c == table_column)
                .unwrap()
            );
        }

        InsertPlan {
            table_id,
            columns: self.columns.clone(),
            index_map: index_map,
        }
    }
}

impl InsertPlan {
    pub(crate) fn insert<T:'static>(
        &self,
        store: &mut Store, 
        index: usize, 
        value: T
    ) -> RowId {
        unsafe {
            let column_id = self.columns[index];
            store.column_mut(column_id).push(value)
        }
    }

    pub(crate) fn cursor<'a>(
        &'a self, 
        store: &'a mut Store,
        id: EntityId,
    ) -> InsertCursor<'a> {
        let mut cursor = InsertCursor {
            id,
            plan: &self,
            store,
            index: 0, 
            rows: Vec::new(),
        };
        /*
        if let Some(id) = id {
            cursor.add_entity(id);
        }
        */
        cursor.add_entity(id);

        cursor
    }
}

impl<'a> InsertCursor<'a> {
    pub(crate) fn add_entity(&mut self, id: EntityId) {
        match self.store.get_entity(id) {
            Some(store_id) => {
                assert_eq!(id, store_id);

                for row_id in self.store.get_entity_columns(id) {
                    self.rows.push(*row_id);
                    self.index += 1;
                }
            },
            None => {}
        }
    }

    pub unsafe fn insert<T:'static>(&mut self, value: T) {
        let index = self.index;
        self.index += 1;

        let row_id = self.plan.insert(self.store, index, value);

        self.rows.push(row_id);
    }

    pub(crate) fn complete(self) -> EntityId {
        let mut columns = Vec::<RowId>::new();

        for index in &self.plan.index_map {
            columns.push(self.rows[*index]);
        }

        match self.store.get_entity(self.id) {
            Some(_) => {
                self.store.insert(self.id, self.plan.table_id, columns)
            },
            None => {
                self.store.push_row(self.id, self.plan.table_id, columns)
            }
        }
    }
}

//
// Insert tuples of components
//

impl<T:Component> Bundle for T {
    fn build(builder: &mut InsertBuilder) {
        builder.add_column::<T>();
    }

    unsafe fn insert(cursor: &mut InsertCursor, this: Self) {
        cursor.insert(this);
    }
}


//
// insert composed of tuples
//

macro_rules! impl_insert_tuple {
    ($($part:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($part:Bundle),*> Bundle for ($($part,)*)
        {
            fn build(builder: &mut InsertBuilder) {
                $(
                    $part::build(builder);
                )*
            }

            unsafe fn insert<'a>(cursor: &mut InsertCursor, value: Self) {
                let ($($part,)*) = value;
                $(
                    $part::insert(cursor, $part);
                )*
            }
        }
    }
}

//impl_query_tuple!();
impl_insert_tuple!(P1,P2);
impl_insert_tuple!(P1,P2,P3);
impl_insert_tuple!(P1,P2,P3,P4);
impl_insert_tuple!(P1,P2,P3,P4,P5);
