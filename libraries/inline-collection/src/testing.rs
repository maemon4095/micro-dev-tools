#![allow(unused_imports)]
mod drop_watcher;
pub(crate) use drop_watcher::*;

pub(crate) trait IsSomeAndExtension<T> {
    fn any(&self, predicate: impl FnOnce(&T) -> bool) -> bool;
}

impl<T> IsSomeAndExtension<T> for Option<T> {
    fn any(&self, predicate: impl FnOnce(&T) -> bool) -> bool {
        self.as_ref().map_or(false, predicate)
    }
}
