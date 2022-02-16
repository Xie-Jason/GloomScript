use std::cell::{Ref, RefCell, RefMut};
use core::fmt::{Debug, Formatter};
use core::hash::{Hash, Hasher};
use core::ops::Deref;
extern crate alloc;
use alloc::rc::Rc;

#[derive(Eq)]
pub struct RefCount<T> {
    rf: Rc<RefCell<T>>,
}

impl<T> RefCount<T> {
    pub fn new(t: T) -> Self {
        return RefCount {
            rf: Rc::new(RefCell::new(t)),
        };
    }
    pub fn inner(&self) -> Ref<T> {
        self.rf.deref().borrow()
    }
    pub fn inner_mut(&self) -> RefMut<T> {
        self.rf.deref().borrow_mut()
    }
}

impl<T: Debug> Debug for RefCount<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner())
    }
}

impl<T: Hash> Hash for RefCount<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner().hash(state)
    }
}

impl<T> Clone for RefCount<T> {
    fn clone(&self) -> Self {
        RefCount {
            rf: self.rf.clone(),
        }
    }
}

impl<T> PartialEq<Self> for RefCount<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.rf, &other.rf)
    }
}

// 仅在单线程环境下使用 just use in single-thread env
unsafe impl<T> Send for RefCount<T> {}

unsafe impl<T> Sync for RefCount<T> {}
