use std::{any::type_name, ops::{Deref, DerefMut}};

use crate::{
    error::Result,
    schedule::{SystemMeta, UnsafeStore}, 
    Store
};

use super::Param;

#[derive(Debug)]
pub struct Res<'w, T> {
    value: &'w T,
}

impl<'w, T:'static> Res<'w, T> {
    pub fn get(&self) -> &T {
        self.value
    }
}

impl<T:Send+'static> Param for Res<'_, T> {
    type Arg<'w, 's> = Res<'w, T>;
    type State = ();

    fn arg<'w, 's>(
        store: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> Result<Res<'w, T>> {
        if let Some(value) = store.get_resource::<T>() {
            Ok(Res {
                value,
            })
        } else {
            Err(format!("Res<{}> is not a resource", type_name::<T>()).into())
        }
    }

    fn init(meta: &mut SystemMeta, store: &mut Store) -> Result<Self::State> {
        if store.contains_resource::<T>() {
            meta.insert_resource(store.get_resource_id::<T>());

            Ok(())
        } else {
            Err(format!("Res<{}> is an unknown resource", type_name::<T>()).into())
        }
    }
}

impl<T:'static> Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T:'static> AsRef<T> for Res<'_, T> {
    fn as_ref(&self) -> &T {
        self.value
    }
}

impl<T: Send + 'static> Param for Option<Res<'_, T>> {
    type Arg<'w, 's> = Option<Res<'w, T>>;
    type State = ();

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> Result<Option<Res<'w, T>>> {
        Ok(world.get_resource::<T>().map(|r| Res { value: r }))
    }

    fn init(meta: &mut SystemMeta, store: &mut Store) -> Result<Self::State> {
        if store.contains_resource::<T>() {
            meta.insert_resource(store.get_resource_id::<T>());
        }

        Ok(())
    }
}

pub struct ResMut<'a, T> {
    value: &'a mut T,
}

impl<T:'static> ResMut<'_, T> {
    pub fn get(&self) -> &T {
        self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<T:'static> Deref for ResMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a, T:'static> DerefMut for ResMut<'_, T> {
    // type Target = T;

    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T: Send+'static> Param for ResMut<'_, T> {
    type Arg<'w, 's> = ResMut<'w, T>;
    type State = ();

    fn init(meta: &mut SystemMeta, store: &mut Store) -> Result<Self::State> {
        if store.contains_resource::<T>() {
            meta.insert_resource_mut(store.get_resource_id::<T>());

            Ok(())
        } else {
            Err(format!("ResMut<{}> is an unknown resource", type_name::<T>()).into())
        }
    }

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> Result<ResMut<'w, T>> {
        Ok(ResMut {
            value: unsafe { world.as_mut().get_resource_mut().unwrap() }
        })
    }
}

impl<T: Send + 'static> Param for Option<ResMut<'_, T>> {
    type Arg<'w, 's> = Option<ResMut<'w, T>>;
    type State = ();

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> Result<Option<ResMut<'w, T>>> {
        unsafe {
            Ok(world.as_mut().get_resource_mut::<T>().map(|r| ResMut { value: r }))
        }
    }

    fn init(meta: &mut SystemMeta, store: &mut Store) -> Result<Self::State> {
        if store.contains_resource::<T>() {
            meta.insert_resource_mut(store.get_resource_id::<T>());
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{core_app::CoreApp, Res, ResMut};

    #[test]
    fn res_plain() {
        let mut app = CoreApp::new();

        app.insert_resource(TestResource(3));

        assert_eq!(3, app.eval(|r: Res<TestResource>| r.0).unwrap());
    }

    #[test]
    fn res_unset() {
        let mut app = CoreApp::new();

        assert_eq!(
            "Res<alloc::string::String> is an unknown resource\n\tin essay_ecs_core::param::res::test::res_unset::{{closure}",
            app.eval(|res: Res<String>| res.as_ref().to_string()).unwrap_err().to_string(),
        );
    }

    #[test]
    fn option_res() {
        let mut app = CoreApp::new();

        app.insert_resource(TestResource(3));

        assert_eq!(3, app.eval(|opt: Option<Res<TestResource>>| {
            match opt {
                Some(r) => r.0,
                None => 0, 
            }
        }).unwrap());

        assert_eq!(0, app.eval(|opt: Option<Res<BogusResource>>| {
            match opt {
                Some(r) => r.0,
                None => 0, 
            }
        }).unwrap());
    }
    
    #[test]
    fn res_mut_unset() {
        let mut app = CoreApp::new();

        assert_eq!(
            "ResMut<alloc::string::String> is an unknown resource\n\tin essay_ecs_core::param::res::test::res_mut_unset::{{closure}}",
            app.eval(|_res: ResMut<String>| "error").unwrap_err().to_string(),
        );
    }

    #[test]
    fn option_res_mut() {
        let mut app = CoreApp::new();

        app.insert_resource(TestResource(3));

        app.eval(|opt: Option<ResMut<TestResource>>| {
            if let Some(mut r) = opt {
                r.0 = 15;
            }
        }).unwrap();

        assert_eq!(15, app.eval(|res: Res<TestResource>| res.0).unwrap());

        app.eval(|opt: Option<ResMut<BogusResource>>| {
            if let Some(mut r) = opt {
                r.0 = 15;
            }
        }).unwrap();
    }

    struct TestResource(usize);
    struct BogusResource(usize);
}
