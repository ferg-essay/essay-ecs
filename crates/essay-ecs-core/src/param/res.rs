use std::ops::{Deref, DerefMut};

use crate::{schedule::{SystemMeta, UnsafeStore}, Store};

use super::Param;

#[derive(Debug)]
pub struct Res<'a, T> {
    value: &'a T,
}

impl<'a, T:'static> Res<'a, T> {
    pub fn get(&self) -> &T {
        self.value
    }
}

impl<T:Send+'static> Param for Res<'_, T> {
    type Arg<'w, 's> = Res<'w, T>;
    type State = ();

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> Res<'w, T> {
        Res {
            value: world.get_resource::<T>().unwrap(),
        }
    }

    fn init(meta: &mut SystemMeta, world: &mut Store) -> Self::State {
        meta.insert_resource(world.get_resource_id::<T>());
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
    ) -> Option<Res<'w, T>> {
        world.get_resource::<T>().map(|r| Res { value: r })
    }

    fn init(meta: &mut SystemMeta, world: &mut Store) -> Self::State {
        if world.contains_resource::<T>() {
            meta.insert_resource(world.get_resource_id::<T>());
        }
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

impl<T:Send+'static> Param for ResMut<'_, T> {
    type Arg<'w, 's> = ResMut<'w, T>;
    type State = ();

    fn init(meta: &mut SystemMeta, world: &mut Store) -> Self::State {
        meta.insert_resource_mut(world.get_resource_id::<T>());
    }

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> ResMut<'w, T> {
        ResMut {
            value: unsafe { world.as_mut().get_resource_mut().unwrap() }
        }
    }
}

impl<T: Send + 'static> Param for Option<ResMut<'_, T>> {
    type Arg<'w, 's> = Option<ResMut<'w, T>>;
    type State = ();

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> Option<ResMut<'w, T>> {
        unsafe {
            world.as_mut().get_resource_mut::<T>().map(|r| ResMut { value: r })
        }
    }

    fn init(meta: &mut SystemMeta, world: &mut Store) -> Self::State {
        if world.contains_resource::<T>() {
            meta.insert_resource_mut(world.get_resource_id::<T>());
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{core_app::CoreApp, Res, ResMut};

    #[test]
    fn res_plain() {
        let mut app = CoreApp::new();

        app.insert_resource(TestResource(3));

        assert_eq!(3, app.eval(|r: Res<TestResource>| r.0));
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
        }));

        assert_eq!(0, app.eval(|opt: Option<Res<BogusResource>>| {
            match opt {
                Some(r) => r.0,
                None => 0, 
            }
        }));
    }

    #[test]
    fn option_res_mut() {
        let mut app = CoreApp::new();

        app.insert_resource(TestResource(3));

        app.eval(|opt: Option<ResMut<TestResource>>| {
            if let Some(mut r) = opt {
                r.0 = 15;
            }
        });

        assert_eq!(15, app.eval(|res: Res<TestResource>| res.0));

        app.eval(|opt: Option<ResMut<BogusResource>>| {
            if let Some(mut r) = opt {
                r.0 = 15;
            }
        });
    }

    struct TestResource(usize);
    struct BogusResource(usize);
}
