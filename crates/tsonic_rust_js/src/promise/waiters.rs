use std::task::Waker;

use slab::Slab;

const INLINE_LINKS: usize = 3;
const END: usize = usize::MAX;

#[derive(Default)]
pub(super) struct Waiters {
    first: Option<Waker>,
    more: Option<Box<Queue>>,
}

struct Queue {
    inline: [Option<Link>; INLINE_LINKS],
    spill: Slab<Link>,
    head: usize,
    tail: usize,
}

struct Link {
    waker: Waker,
    previous: usize,
    next: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaiterKey {
    First,
    Queued(usize),
}

impl Waiters {
    pub(super) fn register(&mut self, waker: &Waker) -> WaiterKey {
        let queued = self.more.as_ref().is_some_and(|more| !more.is_empty());
        if self.first.is_none() && !queued {
            self.first = Some(waker.clone());
            return WaiterKey::First;
        }
        let more = self.more.get_or_insert_with(|| Box::new(Queue::new()));
        WaiterKey::Queued(more.push(waker.clone()))
    }

    pub(super) fn update(&mut self, key: WaiterKey, waker: &Waker) {
        let slot = match key {
            WaiterKey::First => self.first.as_mut(),
            WaiterKey::Queued(index) => self
                .more
                .as_mut()
                .and_then(|more| more.link_mut(index))
                .map(|link| &mut link.waker),
        };
        slot.expect("a registered Promise waiter keeps its slot until removal")
            .clone_from(waker);
    }

    pub(super) fn remove(&mut self, key: WaiterKey) {
        match key {
            WaiterKey::First => {
                self.first = None;
            }
            WaiterKey::Queued(index) => {
                if let Some(more) = self.more.as_mut() {
                    more.remove(index);
                }
            }
        }
    }

    pub(super) fn wake_all(self) {
        if let Some(waker) = self.first {
            waker.wake();
        }
        if let Some(more) = self.more {
            (*more).wake_all();
        }
    }
}

impl Queue {
    fn new() -> Self {
        Self {
            inline: Default::default(),
            spill: Slab::new(),
            head: END,
            tail: END,
        }
    }

    fn is_empty(&self) -> bool {
        self.head == END
    }

    fn link_mut(&mut self, index: usize) -> Option<&mut Link> {
        if index < INLINE_LINKS {
            self.inline[index].as_mut()
        } else {
            self.spill.get_mut(index - INLINE_LINKS)
        }
    }

    fn take(&mut self, index: usize) -> Option<Link> {
        if index < INLINE_LINKS {
            self.inline[index].take()
        } else {
            self.spill.try_remove(index - INLINE_LINKS)
        }
    }

    fn push(&mut self, waker: Waker) -> usize {
        let link = Link {
            waker,
            previous: self.tail,
            next: END,
        };
        let index = match self.inline.iter().position(Option::is_none) {
            Some(index) => {
                self.inline[index] = Some(link);
                index
            }
            None => INLINE_LINKS + self.spill.insert(link),
        };
        match self.link_mut(self.tail) {
            Some(tail) => tail.next = index,
            None => self.head = index,
        }
        self.tail = index;
        index
    }

    fn remove(&mut self, index: usize) {
        let Some(link) = self.take(index) else {
            return;
        };
        match self.link_mut(link.previous) {
            Some(previous) => previous.next = link.next,
            None => self.head = link.next,
        }
        match self.link_mut(link.next) {
            Some(next) => next.previous = link.previous,
            None => self.tail = link.previous,
        }
    }

    fn wake_all(mut self) {
        let mut index = self.head;
        while let Some(link) = self.take(index) {
            index = link.next;
            link.waker.wake();
        }
    }
}
