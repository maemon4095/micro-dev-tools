#![allow(dead_code)]
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

#[derive(Debug, Clone)]
pub(crate) struct DropMarkerState<T> {
    drop_count: usize,
    props: T,
}
impl<T> DropMarkerState<T> {
    pub(crate) fn is_properly_dropped(&self) -> bool {
        self.drop_count == 1
    }

    pub(crate) fn is_illegally_dropped(&self) -> bool {
        self.drop_count > 1
    }

    pub(crate) fn is_leaked(&self) -> bool {
        self.drop_count == 0
    }
}

#[derive(Debug)]
struct DropWatcherProps<T> {
    markers: Vec<DropMarkerState<T>>,
}

#[derive(Debug)]
pub(crate) struct DropWatcher<T> {
    props: Rc<RefCell<DropWatcherProps<T>>>,
}

impl<T> DropWatcher<T> {
    pub(crate) fn new() -> Self {
        Self {
            props: Rc::new(RefCell::new(DropWatcherProps { markers: Vec::new() })),
        }
    }

    pub(crate) fn notify_drop(&self, id: usize) {
        self.props.as_ref().borrow_mut().markers[id].drop_count += 1;
    }

    pub(crate) fn alloc(&self, props: T) -> DropMarker<T> {
        let id = self.props.as_ref().borrow().markers.len();
        self.props.as_ref().borrow_mut().markers.push(DropMarkerState { drop_count: 0, props });
        DropMarker { id, watcher: self }
    }

    pub(crate) fn watch(&self, id: usize) -> Ref<DropMarkerState<T>> {
        Ref::<'_, DropWatcherProps<_>>::map(self.props.as_ref().borrow(), |w| &w.markers[id])
    }

    pub(crate) fn markers(&self) -> Ref<[DropMarkerState<T>]> {
        Ref::<'_, DropWatcherProps<T>>::map(self.props.as_ref().borrow(), |w| w.markers.as_slice())
    }
}

#[derive(Debug)]
pub(crate) struct DropMarker<'a, T> {
    id: usize,
    watcher: &'a DropWatcher<T>,
}

impl<'a, T> Drop for DropMarker<'a, T> {
    fn drop(&mut self) {
        #[cfg(test)]
        {
            let ptr: *const DropWatcher<T> = self.watcher;
            if ptr.is_null() {
                panic!("uninitialized dropwatcher is dropped");
            }
        }
        self.watcher.notify_drop(self.id);
    }
}

impl<'a, T> DropMarker<'a, T> {
    pub(crate) fn props(&self) -> Ref<T> {
        Ref::map(self.watcher.watch(self.id), |s| &s.props)
    }
}

impl<'a, T: PartialEq> PartialEq for DropMarker<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
