use core::fmt;
use std::{borrow::Cow, any::type_name};

use crate::{world::World};

use super::{schedule::SystemId, Phase, phase::DefaultPhase};

pub trait System: Send + Sync + 'static {
    type Out;

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn init(&mut self, meta: &mut SystemMeta, world: &mut World);

    unsafe fn run_unsafe(&mut self, world: &World) -> Self::Out;

    fn run(&mut self, world: &mut World) -> Self::Out {
        unsafe { self.run_unsafe(world) }
    }

    fn flush(&mut self, world: &mut World);
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Priority(u32);

pub struct SystemMeta {
    id: SystemId,
    name: Cow<'static, str>,

    priority: Priority,

    is_exclusive: bool,
    is_flush: bool,
}

pub trait IntoSystem<Out,M>: Sized {
    type System:System<Out=Out>;

    fn into_system(this: Self) -> Self::System;
}

pub struct SystemConfig {
    pub(crate) system: Box<dyn System<Out=()>>,

    pub(crate) phase: Option<Box<dyn Phase>>,
}

pub struct SystemConfigs {
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

impl SystemMeta {
    pub(crate) fn new(id: SystemId, name: &'static str) -> Self {
        Self {
            id,
            name: name.into(),
            priority: Default::default(),
            is_flush: false,
            is_exclusive: false,
        }
    }

    pub fn empty() -> Self {
        Self {
            id: SystemId(0),
            name: "empty".into(),
            priority: Default::default(),
            is_flush: false,
            is_exclusive: false,
        }
    }

    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn set_exclusive(&mut self) {
        self.is_exclusive = true;
    }

    pub fn is_exclusive(&self) -> bool {
        self.is_exclusive
    }

    pub fn set_flush(&mut self) {
        self.is_flush = true;
    }

    pub fn is_flush(&self) -> bool {
        self.is_flush
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }

    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }

    pub fn add_priority(&mut self, delta: u32) {
        self.priority = self.priority.add(delta);
    }

    pub fn sub_priority(&mut self, delta: u32) {
        self.priority = self.priority.sub(delta);
    }
}

impl fmt::Debug for SystemMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemMeta")
         .field("id", &self.id)
         .field("name", &self.name)
         .finish()
    }
}

struct IsSelf;

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

impl Priority {
    const HIGH : Priority = Priority(2000);
    const DEFAULT : Priority = Priority(1000);
    const LOW : Priority = Priority(500);

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn add(&self, value: u32) -> Priority {
        Priority(self.0 + value)
    }

    pub fn sub(&self, value: u32) -> Priority {
        Priority(self.0 - value)
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::DEFAULT
    }
}

impl From<u32> for Priority {
    fn from(value: u32) -> Self {
        Priority(value)
    }
}
