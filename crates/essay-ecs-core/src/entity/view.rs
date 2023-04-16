
//
// Query
//

use std::marker::PhantomData;

use super::{
    {Store, ViewId}, 
    meta::{TableType, ViewTableType, ColumnId}, 
    store::{Row, Component}
};

pub trait View {
    type Item<'a>;

    fn build(builder: &mut ViewBuilder);

    unsafe fn deref<'a, 't>(cursor: &mut ViewCursor<'a, 't>) -> Self::Item<'t>;
}

pub struct ViewCursor<'a, 't> {
    store: &'t Store,
    table: &'a TableType,
    view_table: &'a ViewTableType,
    row: &'a Row,
    cols: &'a Vec<usize>,
    index: usize,
}

pub struct ViewBuilder<'a> {
    store: &'a mut Store, 
    columns: Vec<ColumnId>,
}

pub(crate) struct ViewPlan {
    view: ViewId,
    cols: Vec<usize>,
}

impl ViewPlan {
    pub(crate) fn new_cursor<'a, 't>(
        &'a self, 
        store: &'t Store,
        table: &'a TableType,
        view_row: &'a ViewTableType,
        row: &'a Row
    ) -> ViewCursor<'a, 't> {
        ViewCursor {
            store,
            table,
            row: row,
            view_table: view_row,
            cols: &self.cols,
            index: 0,
        }
    }

    pub(crate) fn view(&self) -> ViewId {
        self.view
    }
}

impl<'a, 't> ViewCursor<'a, 't> {
    pub unsafe fn deref<T:'static>(&mut self) -> &'t T {
        let index = self.view_table.index_map()[self.cols[self.index]];
        self.index += 1;

        let column_id = self.table.columns()[index];
        let row_id = self.row.column_row(index);

        self.store.get_by_id::<T>(column_id, row_id).unwrap()
    }

    pub unsafe fn deref_mut<T:'static>(&mut self) -> &'t mut T {
        let index = self.view_table.index_map()[self.cols[self.index]];
        self.index += 1;

        let column_id = self.table.columns()[index];
        let row_id = self.row.column_row(index);

        self.store.get_mut_by_id(column_id, row_id).unwrap()
    }
}

impl<'a, 't> ViewBuilder<'a> {
    pub(crate) fn new(store: &'a mut Store) -> Self {
        Self {
            store,
            columns: Vec::new(),
        }
    }

    pub fn add_ref<T:'static>(&mut self) {
        let col_id = self.store.add_column::<T>();

        self.columns.push(col_id);
    }

    pub fn add_mut<T:'static>(&mut self) {
        let col_id = self.store.add_column::<T>();

        self.columns.push(col_id);
    }

    pub(crate) fn build(self) -> ViewPlan {
        let view_id = self.store.add_view(&self.columns);
        let view = self.store.view(view_id);

        let cols = self.columns.iter()
            .map(|col_id| view.column_position(*col_id).unwrap())
            .collect();

        ViewPlan {
            view: view_id,
            cols: cols,
        }
    }
}

pub struct ViewIterator<'a, T:View> {
    store: &'a Store,

    view_id: ViewId,
    plan: ViewPlan,

    view_type_index: usize,

    row_index: usize,

    marker: PhantomData<T>,
}

impl<'a, T:View> ViewIterator<'a, T> {
    pub(crate) fn new(
        table: &'a Store, 
        plan: ViewPlan,
    ) -> Self {
        Self {
            store: table,

            view_id: plan.view(),
            plan,

            view_type_index: 0,
            row_index: 0,

            marker: PhantomData,
        }
    }
}

impl<'a, T:View> Iterator for ViewIterator<'a, T>
{
    type Item = T::Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let view = self.store.meta().view(self.view_id);

        while self.view_type_index < view.view_tables().len() {
            let view_table_id = view.view_tables()[self.view_type_index];
            let view_table = self.store.meta().view_table(view_table_id);
            let table_id = view_table.table_id();
            let table = self.store.meta().table(table_id);
            let row_index = self.row_index;
            self.row_index += 1;

            match self.store.get_row_by_type_index(table_id, row_index) {
                Some(row) => {
                    return unsafe { 
                        let mut cursor = self.plan.new_cursor(
                            self.store,
                            table, 
                            view_table,
                            row
                        );
                        
                        Some(T::deref(&mut cursor))
                    }
                }
                None => {},
            };

            self.view_type_index += 1;
            self.row_index = 0;
        }

        None
    }
}

//
// view implementation
//

impl<T:Component> View for &T {
    type Item<'t> = &'t T;

    fn build(builder: &mut ViewBuilder) {
        builder.add_ref::<T>();
    }

    unsafe fn deref<'a, 't>(cursor: &mut ViewCursor<'a, 't>) -> Self::Item<'t> { // Self::Item { // <'a> {
        cursor.deref::<T>()
    }
}

impl<T:Component> View for &mut T {
    type Item<'t> = &'t mut T;

    fn build(builder: &mut ViewBuilder) {
        builder.add_ref::<T>();
    }

    unsafe fn deref<'a, 't>(cursor: &mut ViewCursor<'a, 't>) -> Self::Item<'t> { //<'a> {
        cursor.deref_mut::<T>()
    }
}

//
// View composed of tuples
//

macro_rules! impl_query_tuple {
    ($($part:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($part:View,)*> View for ($($part,)*)
        {
            type Item<'t> = ($(
                <$part as View>::Item<'t>,
            )*);

            fn build(builder: &mut ViewBuilder) {
                $(
                    $part::build(builder);
                )*
            }

            unsafe fn deref<'a,'t>(cursor: &mut ViewCursor<'a, 't>) -> Self::Item<'t> { // <'a> {
                ($(
                    $part::deref(cursor),
                )*)
            }
        }
    }
}

impl_query_tuple!();
impl_query_tuple!(P1,P2);
impl_query_tuple!(P1,P2,P3);
impl_query_tuple!(P1,P2,P3,P4);
impl_query_tuple!(P1,P2,P3,P4,P5);
