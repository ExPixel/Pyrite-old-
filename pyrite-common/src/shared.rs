use std::rc::Rc;
use std::cell::{ Ref, RefMut, RefCell };

pub struct Shared<T: ?Sized> {
    inner: Rc<RefCell<T>>,
}

impl<T: Sized> Shared<T> {
    pub fn new(v: T) -> Shared<T> {
        Shared { inner: Rc::new(RefCell::new(v)) }
    }

    pub fn try_unwrap(this: Shared<T>) -> Result<T, Shared<T>> {
        let inner = this.inner;
        match Rc::try_unwrap(inner) {
            Ok(val) => Ok(val.into_inner()),
            Err(sad) => Err(Shared { inner: sad }),
        }
    }

    pub fn unwrap(this: Shared<T>) -> T {
        match Shared::try_unwrap(this) {
            Ok(unwraped) => unwraped,
            Err(_) => panic!("unable to unwrap shared value (more than one strong reference)"),
        }
    }
}

impl<T: ?Sized> Shared<T> {
    #[inline(always)]
    pub fn borrow(&self) -> Ref<T> {
        self.inner.borrow()
    }

    #[inline(always)]
    pub fn borrow_mut(&self) -> RefMut<T> {
        self.inner.borrow_mut()
    }

    #[inline(always)]
    pub fn with<F>(&self, f: F) where F: FnOnce(Ref<T>) {
        f(self.inner.borrow());
    }

    #[inline(always)]
    pub fn with_mut<F>(&mut self, f: F) where F: FnOnce(RefMut<T>) {
        f(self.inner.borrow_mut());
    }

    pub fn share(this: &Self) -> Self {
        Shared {
            inner: Rc::clone(&this.inner)
        }
    }
}
