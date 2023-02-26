use crate::{gram, Topos};
use crate::{MindBuilder, action::action::ActionBuilder};
use crate::action::action_group::ActionGroup;

#[test]
fn basic_action() {
    let mut builder = MindBuilder::new();
    let mut action = TestAction::new("action");
    action.max(4);
    let mut group = ActionGroup::new(&mut builder);
    let mut action = group.action(gram("a"), action);
    action.on_action(move |a, ctx| {
        a.action(ctx.ticks())
    });
    let ext_source = builder.external_source();
    ext_source.source().to(group.request());
    
    let mut system = builder.build();

    let fiber = ext_source.fiber();

    let mut ptr = action.unwrap();

    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "");
    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "");

    fiber.send((gram("a"), Topos::Nil));
    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "action(1)");
    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "action(2)");
    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "action(3)");
    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "action(4)");
    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "finish(4)");
    system.tick();
    assert_eq!(ptr.write(|a| a.take()), "");
}

#[test]
fn action_choice() {
    let mut builder = MindBuilder::new();
    let mut group = ActionGroup::new(&mut builder);

    let mut a = TestAction::new("a");
    a.max(2);
    let mut a = group.action(gram("a"), a);
    a.on_action(move |a, ctx| {
        a.action(ctx.ticks())
    });

    let mut b = TestAction::new("b");
    b.max(2);
    let mut b = group.action(gram("b"), b);
    b.on_action(move |b, ctx| {
        b.action(ctx.ticks())
    });

    let ext_source = builder.external_source();
    ext_source.source().to(group.request());
    
    let mut system = builder.build();

    let fiber = ext_source.fiber();

    let mut a = a.unwrap();
    let mut b = b.unwrap();

    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|b| b.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "");

    fiber.send((gram("b"), Topos::Nil));
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "b-start(1)");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "b-end(2)");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();

    fiber.send((gram("a"), Topos::Nil));
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-start(1)");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-end(2)");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();

    fiber.send((gram("b"), Topos::Nil));
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "b-start(1)");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "b-end(2)");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
}

#[test]
fn action_competition() {
    let mut builder = MindBuilder::new();
    let mut group = ActionGroup::new(&mut builder);

    let mut a = TestAction::new("a");
    a.max(2);
    let mut a = group.action(gram("a"), a);
    a.on_action(move |a, ctx| {
        a.action(ctx.ticks())
    });

    let mut b = TestAction::new("b");
    b.max(2);
    let mut b = group.action(gram("b"), b);
    b.on_action(move |b, ctx| {
        b.action(ctx.ticks())
    });
    
    let ext_source = builder.external_source();
    ext_source.source().to(group.request());
    
    let mut system = builder.build();

    let fiber = ext_source.fiber();

    let mut a = a.unwrap();
    let mut b = b.unwrap();

    fiber.send((gram("a"), Topos::Nil));
    fiber.send((gram("b"), Topos::Nil));
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-start(1)");
    assert_eq!(b.write(|b| b.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-end(2)");
    assert_eq!(b.write(|a| a.take()), "");

    fiber.send((gram("a"), Topos::Nil));
    fiber.send((gram("b"), Topos::Nil));
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-start(1)");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-end(2)");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();

    fiber.send((gram("b"), Topos::Nil));
    fiber.send((gram("a"), Topos::Nil));
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "b-start(1)");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "b-end(2)");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();

    fiber.send((gram("a"), Topos::Nil));
    fiber.send((gram("b"), Topos::Nil));
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-start(1)");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "a-end(2)");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
    assert_eq!(a.write(|a| a.take()), "");
    assert_eq!(b.write(|a| a.take()), "");
    system.tick();
}

struct TestAction {
    name: String,
    values: Vec<String>,
    count: u64,
    max: u64, 
}

impl TestAction {
    fn new(str: &str) -> Self {
        Self {
            name: String::from(str),
            values: Vec::new(),
            count: 0,
            max: 2,
        }
    }

    fn max(&mut self, time: u64) {
        self.max = time;
    }

    fn add(&mut self, msg: String) {
        self.values.push(msg);
    }

    fn action(&mut self, ticks: u64) -> bool {
        self.count += 1;
        if self.count == 1 {
            self.add(format!("{}-start({})", self.name, self.count));
            true
        } else if self.count >= self.max {
            self.add(format!("{}-end({})", self.name, self.count));
            self.count = 0;
            false
        } else {
            self.add(format!("{}({})", self.name, self.count));
            true
        }
    }

    fn take(&mut self) -> String {
        let value = self.values.join(", ");

        self.values.drain(..);

        value
    }
}