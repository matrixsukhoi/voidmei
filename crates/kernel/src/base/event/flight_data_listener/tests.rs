use super::*;
use crate::base::event::event_payload::EventPayload;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct RecordingListener {
    seen: RefCell<Vec<String>>,
}

impl FlightDataListener for RecordingListener {
    fn on_flight_data(&self, event: &FlightDataEvent) {
        // 订阅方经 getPayload() 读载荷 (Java 唯一主路径)
        self.seen
            .borrow_mut()
            .push(event.get_payload().map_grid.clone());
    }
}

fn mk_event(map_grid: &str) -> FlightDataEvent {
    FlightDataEvent::new(
        EventPayload::builder()
            .map_grid(map_grid.to_string())
            .build(),
    )
}

// 单个监听者按发布顺序收到每次事件的载荷
#[test]
fn test_listener_receives_events_in_order() {
    let l = RecordingListener {
        seen: RefCell::new(vec![]),
    };
    for g in ["F1", "F2", "F3"] {
        l.on_flight_data(&mk_event(g));
    }
    assert_eq!(*l.seen.borrow(), vec!["F1", "F2", "F3"]);
}

// 同一 &FlightDataEvent 引用可发布给多个订阅者 (Java 对象引用语义)
#[test]
fn test_multiple_listeners_share_one_event() {
    let a = RecordingListener {
        seen: RefCell::new(vec![]),
    };
    let b = RecordingListener {
        seen: RefCell::new(vec![]),
    };
    let event = mk_event("G7");
    a.on_flight_data(&event);
    b.on_flight_data(&event);
    assert_eq!(*a.seen.borrow(), vec!["G7"]);
    assert_eq!(*b.seen.borrow(), vec!["G7"]);
}

// trait 须可作 dyn 对象使用 (未来 FlightDataBus 的 Box<dyn FlightDataListener> 注册形态)
#[test]
fn test_dyn_dispatch() {
    struct CountingListener {
        count: Rc<Cell<u32>>,
    }
    impl FlightDataListener for CountingListener {
        fn on_flight_data(&self, _event: &FlightDataEvent) {
            self.count.set(self.count.get() + 1);
        }
    }

    let c1 = Rc::new(Cell::new(0u32));
    let c2 = Rc::new(Cell::new(0u32));
    let listeners: Vec<Box<dyn FlightDataListener>> = vec![
        Box::new(CountingListener { count: c1.clone() }),
        Box::new(CountingListener { count: c2.clone() }),
    ];
    let event = mk_event("H8");
    for l in &listeners {
        l.on_flight_data(&event);
    }
    // 两个监听者各自收到一次
    assert_eq!(c1.get(), 1);
    assert_eq!(c2.get(), 1);
}
