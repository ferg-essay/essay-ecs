use std::{marker::PhantomData, mem, ops::{DerefMut, Deref}};

use essay_ecs_core::{ResMut, Local, Store, prelude::Param, schedule::{SystemMeta, UnsafeWorld}, Res};

// see bevy_ecs/src/event.rs
//
// InEvent renamed to match In<Channel>. 
// Where events are resources, channels are components.
//

pub struct Events<E: Event> {
    events_next: Vec<E>,
    events_prev: Vec<E>,

    ticks: usize,
}

impl<E: Event> Events<E> {
    pub fn send(&mut self, event: E) {
        self.events_next.push(event);
    }

    pub fn update(mut event: ResMut<Events<E>>) {
        event.update_inner()
    }

    fn update_inner(&mut self) {
        mem::swap(&mut self.events_next, &mut self.events_prev);
        self.events_next.drain(..);
        self.ticks += 1;
    }
}

impl<E: Event> Default for Events<E> {
    fn default() -> Self {
        Self { 
            events_next: Default::default(), 
            events_prev: Default::default(),
            ticks: 1,
        }
    }
}

pub struct InEvent<'w, 's, E: Event> {
    events: Res<'w, Events<E>>,
    cursor: Local<'s, InEventCursor<E>>,
}

impl<E: Event> InEvent<'_, '_, E> {
    pub fn iter(&mut self) -> InEventIter<E> {
        InEventIter {
            events: self.events.deref(),
            cursor: self.cursor.deref_mut(),
            marker: PhantomData,
        }
    }
}

pub struct InEventIter<'w, 's, E: Event> {
    events: &'w Events<E>,
    cursor: &'s mut InEventCursor<E>,
    marker: PhantomData<E>,
}

impl<'w, E: Event> Iterator for InEventIter<'w, '_, E> {
    type Item = &'w E;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(self.events)
    }
}

pub struct InEventCursor<E: Event> {
    ticks: usize,
    i_events: usize,
    marker: PhantomData<E>,
}

impl<E: Event> InEventCursor<E> {
    fn next<'a>(&mut self, events: &'a Events<E>) -> Option<&'a E> {
        if self.ticks + 1 < events.ticks {
            self.ticks = events.ticks - 1;
            self.i_events = 0;
        };

        if self.ticks + 1 == events.ticks {
            if self.i_events < events.events_prev.len() {
                let event = &events.events_prev[self.i_events];
                self.i_events += 1;
                return Some(event);
            } else {
                self.ticks += 1;
                self.i_events = 0;
            }
        }

        if self.i_events < events.events_next.len() {
            let event = &events.events_next[self.i_events];
            self.i_events += 1;
            Some(event)
        } else {
            None
        }
    }
}

impl<E: Event> Default for InEventCursor<E> {
    fn default() -> Self {
        Self {
            ticks: 0,
            i_events: 0,
            marker: PhantomData,
        }
    }
}

pub struct OutEvent<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'a, E: Event> OutEvent<'a, E> {
    pub fn send(&mut self, event: E) {
        self.events.send(event);
    }
}

pub trait Event : Send + Sync + 'static {}


// TODO: create #[derive(Param)]

impl<'w, 's, E: Event> Param for InEvent<'w, 's, E> {
    type Arg<'w1, 's1> = InEvent<'w1, 's1, E>;

    type State = (
        <Res<'w, Events<E>> as Param>::State, 
        <Local<'s, InEventCursor<E>> as Param>::State
    );

    fn init(meta: &mut SystemMeta, world: &mut Store) -> Self::State {
        (
            Res::<Events<E>>::init(meta, world),
            Local::<InEventCursor<E>>::init(meta, world)
        )
    }

    fn arg<'w1, 's1>(
        world: &'w1 UnsafeWorld,
        state: &'s1 mut Self::State, 
    ) -> Self::Arg<'w1, 's1> {
        let (e_st, c_st) = state;

        InEvent {
            events: Res::<Events<E>>::arg(world, e_st),
            cursor: Local::<InEventCursor<E>>::arg(world, c_st),
        }
    }
}

// TODO: create #[derive(Param)]

impl<'w, E: Event> Param for OutEvent<'w, E> {
    type Arg<'w1, 's1> = OutEvent<'w1, E>;

    type State = <ResMut<'w, Events<E>> as Param>::State;

    fn init(meta: &mut SystemMeta, world: &mut Store) -> Self::State {
        ResMut::<Events<E>>::init(meta, world)
    }

    fn arg<'w1, 's1>(
        world: &'w1 UnsafeWorld,
        state: &'s1 mut Self::State, 
    ) -> Self::Arg<'w1, 's1> {

        OutEvent {
            events: ResMut::<Events<E>>::arg(world, state),
        }
    }
}

#[cfg(test)]
mod test {
    use essay_ecs_core::core_app::{CoreApp, Core};

    use essay_ecs_core::util::test::TestValues;

    use super::{Event, Events, InEvent};

    #[test]
    fn test_read_no_update() {
        let mut app = CoreApp::new();
        app.init_resource::<Events<TestEvent>>();

        let mut values = TestValues::new();
        let mut ptr = values.clone();

        app.system(Core, move |mut reader: InEvent<TestEvent>| {
            for event in reader.iter() {
                ptr.push(&format!("{:?}", event));
            }
        });

        // no events
        app.tick();
        assert_eq!(values.take(), "");

        // event read once
        app.resource_mut::<Events<TestEvent>>().send(TestEvent(1));
        app.tick();
        assert_eq!(values.take(), "TestEvent(1)");
        app.tick();
        assert_eq!(values.take(), "");

        // multiple events
        app.resource_mut::<Events<TestEvent>>().send(TestEvent(2));
        app.resource_mut::<Events<TestEvent>>().send(TestEvent(3));
        app.tick();
        assert_eq!(values.take(), "TestEvent(2), TestEvent(3)");
        app.tick();
        assert_eq!(values.take(), "");
    }

    #[test]
    fn test_read_update() {
        let mut app = CoreApp::new();
        app.init_resource::<Events<TestEvent>>();

        let mut values = TestValues::new();
        let mut ptr = values.clone();

        app.system(Core, move |mut reader: InEvent<TestEvent>| {
            for event in reader.iter() {
                ptr.push(&format!("{:?}", event));
            }
        });

        // no events
        app.tick();
        assert_eq!(values.take(), "");

        app.resource_mut::<Events<TestEvent>>().update_inner();
        app.tick();
        assert_eq!(values.take(), "");

        app.resource_mut::<Events<TestEvent>>().update_inner();
        app.tick();
        assert_eq!(values.take(), "");

        // event read after update
        app.resource_mut::<Events<TestEvent>>().send(TestEvent(1));
        app.resource_mut::<Events<TestEvent>>().update_inner();
        app.tick();
        assert_eq!(values.take(), "TestEvent(1)");
        app.resource_mut::<Events<TestEvent>>().update_inner();
        app.tick();
        assert_eq!(values.take(), "");

        // two updates make event inaccessible
        app.resource_mut::<Events<TestEvent>>().send(TestEvent(2));
        app.resource_mut::<Events<TestEvent>>().update_inner();
        app.resource_mut::<Events<TestEvent>>().update_inner();
        app.tick();
        assert_eq!(values.take(), "");
        app.resource_mut::<Events<TestEvent>>().update_inner();
        app.tick();
        assert_eq!(values.take(), "");

        app.resource_mut::<Events<TestEvent>>().send(TestEvent(3));
        app.tick();
        assert_eq!(values.take(), "TestEvent(3)");
        app.tick();
        assert_eq!(values.take(), "");
    }

    #[derive(Debug)]
    pub struct TestEvent(usize);

    impl Event for TestEvent {}
}
