use crate::{
    World, Schedule, IntoSystemConfig, 
    schedule::{Phase, IntoPhaseConfigs, ScheduleLabel}, entity::{View, ViewIterator}, Schedules
};


pub struct BaseApp {
    world: World,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum BaseSchedule {
    Main,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum BasePhases {
    First,
    Main,
    Last,
}

impl BaseApp {
    pub fn new() -> Self {
        BaseApp::default()
    }

    pub fn empty() -> Self {
        let mut world = World::new();

        world.init_resource::<Schedules>();

        BaseApp {
            world: world,
        }
    }

    pub fn add_system<M>(
        &mut self, 
        into_system: impl IntoSystemConfig<M>
    ) -> &mut Self
    {
        self.resource_mut::<Schedules>().add_system(
            &BaseSchedule::Main,
            into_system
        );
    
        self
    }

    pub fn get_resource<T:'static>(&mut self) -> Option<&T> {
        self.world.get_resource::<T>()
    }

    pub fn get_mut_resource<T:'static>(&mut self) -> Option<&mut T> {
        self.world.get_resource_mut::<T>()
    }

    pub fn resource<T:'static>(&mut self) -> &T {
        self.world.get_resource::<T>().expect("unassigned resource")
    }

    pub fn resource_mut<T:'static>(&mut self) -> &mut T {
        self.world.get_resource_mut::<T>().expect("unassigned resource")
    }

    pub fn insert_resource<T:'static>(&mut self, value: T) {
        self.world.insert_resource(value);
    }

    pub fn query<Q:View>(&mut self) -> ViewIterator<Q> {
        self.world.query()
    }

    pub fn get_mut_schedule(&mut self, label: &dyn ScheduleLabel) -> Option<&mut Schedule> {
        self.world.resource_mut::<Schedules>().get_mut(label)
    }

    pub fn tick(&mut self) -> &mut Self {
        self.world.run_schedule(BaseSchedule::Main);

        self
    }
}

impl Default for BaseApp {
    fn default() -> Self {
        let mut app = BaseApp::empty();

        app.insert_resource(Schedules::default());
        app.resource_mut::<Schedules>()
            .insert(BaseSchedule::Main, BasePhases::main_schedule());

        app
    }
}

impl BasePhases {
    fn main_schedule() -> Schedule {
        let mut schedule = Schedule::new();

        schedule.set_default_phase(Self::Main);

        schedule.add_phases((
            Self::First,
            Self::Main,
            Self::Last,
        ).chained());

        schedule
    }
}

impl Phase for BasePhases {
    fn box_clone(&self) -> Box<dyn Phase> {
        Box::new(Clone::clone(self))
    }
}

impl ScheduleLabel for BaseSchedule {
    fn box_clone(&self) -> Box<dyn ScheduleLabel> {
        Box::new(Clone::clone(self))
    }
}
