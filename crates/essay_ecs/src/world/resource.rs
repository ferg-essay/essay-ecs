use crate::{entity::prelude::{Table}, type_meta::TypeMetas};

struct IsResource;

pub struct Resources<'w> {
    types: TypeMetas,
    table: Table<'w>,
    //resources: Vec<RowMeta>,
}

impl<'w> Resources<'w> {
    pub fn new() -> Self {
        Self {
            types: TypeMetas::new(),
            table: Table::new(),
            //resources: Vec::new(),
        }
    }

    pub fn get_by_type<T:'static>(&mut self) -> Option<&T> {
        todo!();
        /*
        let type_id = self.types.add_type::<T>();

        let en_ref = self.table.create_ref::<T>(type_id.index() as u32);

        //self.table.get(&self.create_ref::<T>())
        self.table.get(&en_ref)
        */
    }

    pub fn get_mut_by_type<T:'static>(&mut self) -> Option<&mut T> {
        todo!();
        /*
        let type_id = self.types.add_type::<T>();

        let en_ref = self.table.create_ref::<T>(type_id.index() as u32);

        //self.table.get(&self.create_ref::<T>())
        self.table.get_mut(&en_ref)
        */
    }
}

#[cfg(test)]
mod tests {
    use super::Resources;

    #[test]
    fn set_get() {
        /*
        let mut resources = Resources::new();

        let res_a = resources.set(TestA(1));
        assert_eq!(resources.get(&res_a), Some(&TestA(1)));

        let res_b = resources.set(TestB(2));
        assert_eq!(resources.get(&res_b), Some(&TestB(2)));

        let res_a2 = resources.set(TestA(3));
        assert_eq!(resources.get(&res_a2), Some(&TestA(3)));
        assert_eq!(resources.get(&res_a), Some(&TestA(3)));
        */
    }

    #[derive(PartialEq, Debug)]
    struct TestA(u32);

    #[derive(PartialEq, Debug)]
    struct TestB(u32);
}
