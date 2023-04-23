use std::{any::type_name};

use crate::{world::{World}, schedule::{SystemMeta, Phase, DefaultPhase, UnsafeWorld}};

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct SystemId(pub(crate) usize);

pub trait System: Send + Sync + 'static {
    type Out;

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn init(&mut self, meta: &mut SystemMeta, world: &mut World);

    unsafe fn run_unsafe(&mut self, world: &UnsafeWorld) -> Self::Out;

    fn run(&mut self, world: &mut UnsafeWorld) -> Self::Out {
        unsafe { 
            self.run_unsafe(&world)
        }
    }

    fn flush(&mut self, world: &mut World);
}

pub trait IntoSystem<Out,M>: Sized {
    type System:System<Out=Out>;

    fn into_system(this: Self) -> Self::System;
}

pub struct SystemConfig {
    pub(crate) system: Box<dyn System<Out=()>>,

    pub(crate) phase: Option<Box<dyn Phase>>,
}

pub struct _SystemConfigs {
    sets: Vec<SystemConfig>,
}

pub trait IntoSystemConfig<M>: Sized {
    fn into_config(self) -> SystemConfig;

    fn phase(self, phase: impl Phase) -> SystemConfig {
        let mut config = self.into_config();
        config.phase = Some(Box::new(phase));
        config
    }

    fn no_phase(self) -> SystemConfig {
        let mut config = self.into_config();
        config.phase = None;
        config
    }
}

impl SystemId {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl<S,Out> IntoSystem<Out,()> for S
where
    S: System<Out=Out>
{
    type System = S;

    fn into_system(this: Self) -> Self::System {
        this
    }
}

impl SystemConfig {
    fn new(system: Box<dyn System<Out=()>>) -> Self {
        Self {
            system,
            phase: Some(Box::new(DefaultPhase)),
        }
    }
}

//struct IsSelf;

impl IntoSystemConfig<()> for SystemConfig
{
    fn into_config(self) -> SystemConfig {
        self
    }
}

impl IntoSystemConfig<()> for Box<dyn System<Out=()>>
{
    fn into_config(self) -> SystemConfig {
        SystemConfig::new(self)
    }
}

impl<S,M> IntoSystemConfig<M> for S
where
    S: IntoSystem<(), M>
{
    fn into_config(self) -> SystemConfig {
        SystemConfig::new(Box::new(IntoSystem::into_system(self)))
    }
}
