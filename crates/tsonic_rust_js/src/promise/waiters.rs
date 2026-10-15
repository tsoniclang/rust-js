use std::task::Waker;

use slab::Slab;

const INLINE_FAN_OUT: usize = 3;

#[derive(Default)]
pub(super) struct Waiters {
    first: Option<Waker>,
    more: Option<Box<FanOut>>,
}

struct FanOut {
    inline: [Option<Waker>; INLINE_FAN_OUT],
    spill: Slab<Waker>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaiterKey {
    First,
    Inline(usize),
    Spill(usize),
}

impl Waiters {
    pub(super) fn register(&mut self, waker: &Waker) -> WaiterKey {
        if self.first.is_none() {
            self.first = Some(waker.clone());
            return WaiterKey::First;
        }
        let more = self.more.get_or_insert_with(|| {
            Box::new(FanOut {
                inline: Default::default(),
                spill: Slab::new(),
            })
        });
        if let Some(index) = more.inline.iter().position(Option::is_none) {
            more.inline[index] = Some(waker.clone());
            return WaiterKey::Inline(index);
        }
        WaiterKey::Spill(more.spill.insert(waker.clone()))
    }

    pub(super) fn update(&mut self, key: WaiterKey, waker: &Waker) {
        let slot = match key {
            WaiterKey::First => self.first.as_mut(),
            WaiterKey::Inline(index) => self
                .more
                .as_mut()
                .and_then(|more| more.inline[index].as_mut()),
            WaiterKey::Spill(index) => self
                .more
                .as_mut()
                .and_then(|more| more.spill.get_mut(index)),
        };
        slot.expect("a registered Promise waiter keeps its slot until removal")
            .clone_from(waker);
    }

    pub(super) fn remove(&mut self, key: WaiterKey) {
        match key {
            WaiterKey::First => {
                self.first = None;
            }
            WaiterKey::Inline(index) => {
                if let Some(more) = self.more.as_mut() {
                    more.inline[index] = None;
                }
            }
            WaiterKey::Spill(index) => {
                if let Some(more) = self.more.as_mut() {
                    more.spill.try_remove(index);
                }
            }
        }
    }

    pub(super) fn wake_all(self) {
        if let Some(waker) = self.first {
            waker.wake();
        }
        if let Some(more) = self.more {
            let FanOut { inline, spill } = *more;
            for waker in inline.into_iter().flatten() {
                waker.wake();
            }
            for (_, waker) in spill {
                waker.wake();
            }
        }
    }
}
