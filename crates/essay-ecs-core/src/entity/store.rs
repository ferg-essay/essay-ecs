use std::sync::{Arc, Mutex};

use super::column::{Column, RowId};
use super::bundle::{InsertBuilder, Bundle, InsertPlan};
use super::ViewId;
use super::table::{Table, TableRow};
use super::view::{View, ViewIterator, ViewBuilder, ViewPlan};
use super::meta::{StoreMeta, ColumnId, TableId, ViewType};

#[derive (Debug, Copy, Clone, PartialEq, Hash, Eq, PartialOrd, Ord)]
pub struct ComponentId(usize);

#[derive(Debug,Clone,Copy,PartialEq,Hash,PartialOrd,Eq)]
pub struct EntityId(u32, u32);

#[derive(Debug)]
pub struct Entity {
    id: EntityId,
    table: TableId,
    row: RowId,
}

impl Entity {
    fn empty(len: usize) -> Entity {
        Self {
            id: EntityId(len as u32, EntityId::FREE_MASK),
            table: TableId::UNSET,
            row: RowId::UNSET,
        }
    }

    fn is_alloc(&self) -> bool {
        self.table != TableId::UNSET
    }
}

struct EntityAlloc {
    capacity: usize,

    free_list: Vec<EntityId>, 
}

pub trait Component: Send + Sync + 'static {}

//
// implementation
//

pub struct EntityStore {
    meta: StoreMeta,

    columns: Vec<Column>,

    tables: Vec<Table>,

    entities: Vec<Entity>,

    free_list: Arc<Mutex<EntityAlloc>>,
}

impl EntityStore {
    pub fn new() -> Self {
        let mut store = Self {
            meta: StoreMeta::new(),

            columns: Vec::new(),
            
            tables: Vec::new(),

            entities: Vec::new(),

            free_list: Arc::new(Mutex::new(EntityAlloc::new())),
        };

        store.add_table(Vec::new());

        store
    }

    pub(crate) fn meta(&self) -> &StoreMeta {
        &self.meta
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    //
    // Column
    //

    pub(crate) fn column_mut(&mut self, column_id: ColumnId) -> &mut Column {
        &mut self.columns[column_id.index()]
    }
    
    pub(crate) fn add_column<T:'static>(&mut self) -> ColumnId {
        let column_id = self.meta.add_column::<T>();

        if column_id.index() < self.columns.len() {
            return column_id;
        }

        assert_eq!(column_id.index(), self.columns.len());

        self.columns.push(Column::new::<T>(&mut self.meta));
        
        column_id
    }

    //
    // row (entity)
    //

    pub fn get<T:'static>(&self, id: EntityId) -> Option<&T> {
        match self.meta().get_column::<T>() {
            Some(column_id) => {
                let entity = self.entities.get(id.index())?;
                let table = &self.tables[entity.table.index()];
                let row = table.get(entity.row)?;

                let index = table.position(column_id)?;

                unsafe {
                    self.get_by_id(column_id, row.column(index))
                }
            },
            None => None,
        }
    }

    pub fn get_mut<T:'static>(&mut self, id: EntityId) -> Option<&mut T> {
        let column_id = self.meta().get_column::<T>()?;
        let entity = self.entities.get(id.index())?;
        let table = &self.tables[entity.table.index()];
        let row = table.get(entity.row)?;

        let index = table.position(column_id)?;

        unsafe {
            self.get_mut_by_id(column_id, row.column(index))
        }
    }

    pub(crate) fn alloc_entity_id(&mut self) -> EntityId {
        self.free_list.lock().unwrap().alloc()
    }

    pub fn spawn_empty(&mut self) -> EntityId {
        let id = self.alloc_entity_id();

        self.spawn_empty_id(id);

        id
    }

    pub fn spawn<T:Bundle>(&mut self, value: T) -> EntityId {
        let plan = self.insert_plan::<T>();

        let id = self.alloc_entity_id();

        self.spawn_with_plan(plan, id, value)
    }

    pub(crate) fn spawn_id<T:Bundle>(&mut self, id: EntityId, value: T) -> EntityId {
        let plan = self.insert_plan::<T>();

        self.spawn_with_plan(plan, id, value)
    }

    pub(crate) fn insert_plan<T:Bundle>(&mut self) -> InsertPlan {
        let mut builder = InsertBuilder::new(self);

        T::build(&mut builder);

        builder.build()
    }

    pub(crate) fn spawn_with_plan<T:Bundle>(
        &mut self, 
        plan: InsertPlan, 
        id: EntityId,
        value: T
    ) -> EntityId {
        let mut cursor = plan.cursor(self, id);

        unsafe {
            T::insert(&mut cursor, value);
        }

        cursor.complete()
    }

    pub(crate) fn extend<T:Bundle>(&mut self, id: EntityId, value: T) -> EntityId {
        let mut builder = InsertBuilder::new(self);

        builder.add_entity(id);

        T::build(&mut builder);

        let plan = builder.build();

        let mut cursor = plan.cursor(self, id);

        unsafe {
            T::insert(&mut cursor, value);
        }
        
        cursor.complete()
    }

    pub(crate) fn add_table(&mut self, cols: Vec<ColumnId>) -> TableId {
        let table_id = self.meta.add_table(cols);

        if table_id.index() < self.tables.len() {
            return table_id;
        }

        let meta = self.meta.table(table_id);

        self.tables.push(Table::new(table_id, meta.clone()));
        
        table_id
    }

    pub(crate) fn insert_or_spawn(
        &mut self, 
        id: EntityId, 
        table_id: TableId, 
        columns: Vec<RowId>
    ) -> EntityId {
        if id.index() < self.entities.len() && self.entities[id.index()].is_alloc()  {
            self.insert(id, table_id, columns)
        } else {
            self.push_row(id, table_id, columns)
        }
    }

    pub(crate) fn insert(
        &mut self, 
        id: EntityId, 
        table_id: TableId, 
        columns: Vec<RowId>
    ) -> EntityId {
        self.remove_table_row(id);

        let table = &mut self.tables[table_id.index()];
        let row = table.push(id, columns);

        let entity = Entity { 
            id,
            table: table_id,
            row,
        };

        self.entities[id.index()] = entity;
        
        id // TODO: next()
    }

    fn remove_table_row(&mut self, id: EntityId) {
        let entity = &self.entities[id.index()];

        assert_eq!(entity.id, id);

        let table = &mut self.tables[entity.table.index()];

        table.remove(entity.row);

    }

    pub(crate) fn despawn(&mut self, id: EntityId) {
        self.remove_table_row_and_columns(id);

        let entity = &mut self.entities[id.index()];

        assert_eq!(entity.id, id);

        entity.id = id.free();
        entity.table = TableId::UNSET;
        entity.row = RowId::UNSET;

        self.free_list.lock().unwrap().free(entity.id);
    }

    fn remove_table_row_and_columns(&mut self, id: EntityId) {
        let entity = &self.entities[id.index()];

        assert_eq!(entity.id, id);

        let table = &mut self.tables[entity.table.index()];
        let table_row = table.get(entity.row).unwrap();

        for (col_id, col_row) in 
            table.meta().columns().iter().zip(table_row.columns()) {
            let col = &mut self.columns[col_id.index()];
            col.remove(*col_row);
        }

        table.remove(entity.row);
    }

    pub(crate) fn push_row(
        &mut self,
        id: EntityId, 
        table_id: TableId, 
        columns: Vec<RowId>
    ) -> EntityId {
        let table = &mut self.tables[table_id.index()];

        let row = table.push(id, columns);
        
        let entity = Entity { 
            id,
            table: table_id,
            row,
        };

        self.set_entity(entity);

        id
    }

    pub(crate) fn spawn_empty_id(
        &mut self,
        id: EntityId,
    ) -> EntityId {
        let table = &mut self.tables[0];

        let row = table.push(id, Vec::new());
        
        let entity = Entity { 
            id,
            table: table.id(),
            row,
        };

        self.set_entity(entity);

        id
    }

    fn set_entity(&mut self, entity: Entity) {
        let id = entity.id;
        assert!(id.is_alloc());

        if id.index() < self.entities.len() {
            // TODO:
            // assert_eq!(self.entities[id.index()].id.alloc(), id);
            self.entities[id.index()] = entity;
        } else {
            while self.entities.len() < id.index() {
                self.entities.push(Entity::empty(self.entities.len()));
            }

            self.entities.push(entity);
        }
    }

    //
    // View
    //

    pub fn iter_view<'a,T:View>(&mut self) -> ViewIterator<'_,T> {
        let plan = self.view_plan::<T>();
        
        unsafe { self.iter_view_with_plan(plan) }
    }

    pub(crate) fn view_plan<T:View>(&mut self) -> ViewPlan {
        let mut builder = ViewBuilder::new(self);

        T::build(&mut builder);

        builder.build()
    }

    pub(crate) unsafe fn iter_view_with_plan<T:View>(
        &self, 
        plan: ViewPlan
    ) -> ViewIterator<'_,T> {
        ViewIterator::new(self, plan)
    }

    pub(crate) fn view(&self, view_id: ViewId) -> &ViewType {
        self.meta.view(view_id)
    }

    pub(crate) fn add_view(&mut self, columns: &Vec<ColumnId>) -> ViewId {
        self.meta.add_view(columns)
    }

    pub(crate) unsafe fn get_by_id<T:'static>(
        &self, 
        column_id: ColumnId, 
        row_id: RowId
    ) -> Option<&T> {
        self.columns[column_id.index()].get(row_id)
    }

    pub(crate) unsafe fn get_mut_by_id<T:'static>(
        &self, 
        column_id: ColumnId, 
        row_id: RowId
    ) -> Option<&mut T> {
        self.columns[column_id.index()].get_mut(row_id)
    }

    pub(crate) fn get_row_by_type_index(
        &self, 
        table_id: TableId, 
        row_index: usize
    ) -> Option<&TableRow> {
        self.tables[table_id.index()].get_by_index(row_index)
    }

    pub(crate) fn entity_column_ids(&self, id: EntityId) -> &Vec<ColumnId> {
        let entity = &self.entities[id.index()];
        let table = &self.tables[entity.table.index()];

        table.meta().columns()
    }

    pub(crate) fn get_table(&self, id: TableId) -> Option<&Table> {
        if id == TableId::UNSET {
            None
        } else {
            Some(&self.tables[id.index()])
        }
    }

    pub(crate) fn get_entity_columns(&self, id: EntityId) -> Option<&Vec<RowId>> {
        assert!(id.is_alloc());
        let entity = &self.entities[id.index()];

        match self.get_table(entity.table) {
            Some(table) => {
                Some(table.get(entity.row).unwrap().columns())
            },
            None => None
        }
    }

    pub(crate) fn get_entity(&self, id: EntityId) -> Option<EntityId> {
        if id.index() < self.entities.len() {
            Some(id)
        } else {
            None
        }
    }
}

impl EntityAlloc {
    fn new() -> Self {
        Self {
            capacity: 0,
            free_list: Default::default(),
        }
    }

    pub fn alloc(&mut self) -> EntityId {
        if let Some(entity) = self.free_list.pop() {
            entity.alloc()
        } else {
            let index = self.capacity;
            self.capacity = index + 1;

            EntityId::new(index)
        }
    }

    fn free(&mut self, id: EntityId) {
        assert!(! id.is_alloc());

        self.free_list.push(id);
    } 
}

impl EntityId {
    const FREE_MASK : u32 = 0x8000_0000;

    pub(crate) fn new(index: usize) -> Self {
        Self(index as u32, 0)
    }

    pub(crate) fn index(&self) -> usize {
        self.0 as usize
    }

    pub(crate) fn _gen(&self) -> u32 {
        self.1
    }

    pub(crate) fn is_alloc(&self) -> bool {
        self.1 & Self::FREE_MASK == 0
    }

    pub(crate) fn free(&self) -> EntityId {
        assert!(self.is_alloc());

        EntityId(self.0, (self.1 + 1) | Self::FREE_MASK)
    }

    pub(crate) fn alloc(&self) -> EntityId {
        assert!(! self.is_alloc());

        EntityId(self.0, self.1 & !Self::FREE_MASK)
    }

    pub(crate) fn _is_next_alloc(&self, id: EntityId) -> bool {
        (self.1 & Self::FREE_MASK) != 0
        && (self.1 & !Self::FREE_MASK) == id.1
    }
}

impl From<ColumnId> for ComponentId {
    fn from(id: ColumnId) -> Self {
        ComponentId(id.index())
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::{bundle::InsertCursor, Component};

    use super::{EntityStore, InsertBuilder, Bundle};

    #[test]
    fn spawn() {
        let mut store = EntityStore::new();
        assert_eq!(store.len(), 0);

        store.spawn(TestA(1));
        assert_eq!(store.len(), 1);

        let mut values : Vec<String> = store.iter_view::<&TestA>()
            .map(|t| format!("{:?}", t))
            .collect();
        assert_eq!(values.join(","), "TestA(1)");

        store.spawn(TestB(10000));
        assert_eq!(store.len(), 2);

        values = store.iter_view::<&TestB>().map(|t| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestB(10000)");

        values = store.iter_view::<&TestA>().map(|t| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestA(1)");

        store.spawn(TestB(100));
        assert_eq!(store.len(), 3);

        values = store.iter_view::<&TestA>().map(|t: &TestA| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestA(1)");

        values = store.iter_view::<&TestB>().map(|t: &TestB| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestB(10000),TestB(100)");

        for entity in store.iter_view::<&mut TestB>() {
            entity.0 += 1;
        }
        
        values = store.iter_view::<&TestB>().map(|t: &TestB| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestB(10001),TestB(101)");
    }

    #[test]
    fn entity_get() {
        let mut store = EntityStore::new();
        assert_eq!(store.len(), 0);

        let id_0 = store.spawn(TestA(1000));
        assert_eq!(store.len(), 1);
        assert_eq!(id_0.index(), 0);

        assert_eq!(store.get::<TestA>(id_0), Some(&TestA(1000)));
        assert_eq!(store.get::<TestB>(id_0), None);

        let id_1 = store.spawn(TestB(1001));
        assert_eq!(store.len(), 2);
        assert_eq!(id_1.index(), 1);

        assert_eq!(store.get::<TestA>(id_1), None);
        assert_eq!(store.get::<TestB>(id_1), Some(&TestB(1001)));

        let id_2 = store.spawn(TestA(1002));
        assert_eq!(store.len(), 3);
        assert_eq!(id_2.index(), 2);

        assert_eq!(store.get::<TestA>(id_2), Some(&TestA(1002)));
        assert_eq!(store.get::<TestB>(id_2), None);

        assert_eq!(store.get::<TestA>(id_0), Some(&TestA(1000)));
        assert_eq!(store.get::<TestB>(id_0), None);
    }

    #[test]
    fn push_type() {
        let mut store = EntityStore::new();
        assert_eq!(store.len(), 0);

        store.spawn::<TestA>(TestA(1));
        //table.push(TestC(1));
        assert_eq!(store.len(), 1);

        let values : Vec<String> = store.iter_view::<&TestA>()
            .map(|t| format!("{:?}", t))
            .collect();
        assert_eq!(values.join(","), "TestA(1)");
    }

    #[test]
    fn push_tuple() {
        let mut store = EntityStore::new();
        assert_eq!(store.len(), 0);

        store.spawn((TestA(1),TestB(2)));
        store.spawn((TestB(3),TestA(4)));
        
        assert_eq!(store.len(), 2);

        let mut values : Vec<String> = store.iter_view::<&TestA>()
            .map(|t| format!("{:?}", t))
            .collect();
        assert_eq!(values.join(","), "TestA(1),TestA(4)");

        values = store.iter_view::<&TestB>().map(|t: &TestB| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestB(2),TestB(3)");

        values = store.iter_view::<(&TestA,&TestB)>().map(|v| format!("{:?}", v)).collect();
        assert_eq!(values.join(","), "(TestA(1), TestB(2)),(TestA(4), TestB(3))");
    }

    #[test]
    fn insert_extend() {
        let mut store = EntityStore::new();
        assert_eq!(store.len(), 0);

        let id = store.spawn(TestA(1));
        store.spawn(TestA(2));
        store.spawn(TestA(3));

        let mut values : Vec<String> = store.iter_view::<&TestA>()
            .map(|t: &TestA| format!("{:?}", t))
            .collect();
        assert_eq!(values.join(","), "TestA(1),TestA(2),TestA(3)");

        store.extend(id, TestB(10));

        values = store.iter_view::<&TestA>().map(|t: &TestA| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestA(2),TestA(3),TestA(1)");

        values = store.iter_view::<&TestB>().map(|t: &TestB| format!("{:?}", t)).collect();
        assert_eq!(values.join(","), "TestB(10)");

        values = store.iter_view::<(&TestA,&TestB)>().map(|v| format!("{:?}", v)).collect();
        assert_eq!(values.join(","), "(TestA(1), TestB(10))");
    }

    #[test]
    fn despawn() {
        let mut store = EntityStore::new();
        assert_eq!(store.len(), 0);

        let id_0 = store.spawn::<TestA>(TestA(1));
        store.spawn::<TestA>(TestA(2));
        store.spawn::<TestA>(TestA(3));

        store.despawn(id_0);

        let values : Vec<String> = store.iter_view::<&TestA>()
            .map(|t| format!("{:?}", t))
            .collect();
        assert_eq!(values.join(","), "TestA(2),TestA(3)");

        store.spawn::<TestA>(TestA(4));
        store.spawn::<TestA>(TestA(5));

        let values : Vec<String> = store.iter_view::<&TestA>()
            .map(|t| format!("{:?}", t))
            .collect();
        assert_eq!(values.join(","), "TestA(4),TestA(2),TestA(3),TestA(5)");
    }

    #[derive(Debug, PartialEq)]
    struct TestA(u32);

    #[derive(Debug, PartialEq)]
    struct TestB(u16);

    #[derive(Debug, PartialEq)]
    struct TestC(u32);

    trait TestComponent:'static {}
    
    impl Component for TestA {}
    impl Component for TestB {}
    
    impl Bundle for TestC {
        fn build(builder: &mut InsertBuilder) {
            builder.add_column::<TestC>()
        }

        unsafe fn insert(cursor: &mut InsertCursor, value: Self) {
            cursor.insert(value);
        }
    }
}