///
/// See Bevy label.rs for original idea
/// 
use std::{hash::{Hash, Hasher}, any::Any};

pub trait DynLabel : 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_dyn_eq(&self) -> &dyn DynLabel;
    fn dyn_eq(&self, other: &dyn DynLabel) -> bool;
    fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl<T> DynLabel for T
where
    T: Any + Eq + Hash
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_dyn_eq(&self) -> &dyn DynLabel {
        self
    }

    fn dyn_eq(&self, other: &dyn DynLabel) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<T>() {
            return self == other;
        }
        false
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        T::hash(self, &mut state);
        self.type_id().hash(&mut state);
    }
}

