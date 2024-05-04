use crate::{system::System, IntoSystem};

use super::Phase;

pub struct SystemConfigs {
    pub(crate) systems: Vec::<SystemConfig>,
}

impl SystemConfigs {
    fn new(system: Box<dyn System<Out=()>>) -> Self {
        Self {
            systems: vec![SystemConfig::new(system)]
        }
    }

    fn phase(mut self, phase: impl Phase) -> SystemConfigs {
        let phase = Box::new(phase);

        for system in &mut self.systems {
            system.phases.push(phase.box_clone());
        }

        self
    }

    fn run_if<N>(self, _condition: impl IntoSystem<bool, N>) -> SystemConfigs {
        /*
        config.conditions.push(Box::new(IntoSystem::into_system(condition)));
        config
        */

        todo!();

        //self
    }
}

pub(crate) struct SystemConfig {
    pub(crate) system: Box<dyn System<Out = ()>>,

    pub(crate) phases: Vec<Box<dyn Phase>>,

    pub(crate) conditions: Vec<Box<dyn System<Out = bool>>>,
}

impl SystemConfig {
    fn new(system: Box<dyn System<Out=()>>) -> Self {
        Self {
            system,
            phases: Vec::new(),
            conditions: Vec::new(),
        }
    }
}

pub trait IntoSystemConfig<M> : Sized {
    fn into_config(self) -> SystemConfigs;

    fn phase(self, phase: impl Phase) -> SystemConfigs {
        self.into_config().phase(phase)
    }

    fn run_if<N>(self, condition: impl IntoSystem<bool, N>) -> SystemConfigs {
        self.into_config().run_if(condition)
    }
}

//struct IsSelf;

impl IntoSystemConfig<()> for SystemConfigs {
    fn into_config(self) -> SystemConfigs {
        self
    }
}

impl IntoSystemConfig<()> for Box<dyn System<Out=()>> {
    fn into_config(self) -> SystemConfigs {
        SystemConfigs::new(self)
    }
}

impl<S,M> IntoSystemConfig<M> for S
where
    S: IntoSystem<(), M>
{
    fn into_config(self) -> SystemConfigs {
        SystemConfigs::new(Box::new(IntoSystem::into_system(self)))
    }
}
