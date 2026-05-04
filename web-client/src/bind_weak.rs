use std::rc::{Rc, Weak};

pub fn bind_weak_fn0<T, O>(x: Weak<T>, f: impl Fn(Rc<T>) -> O) -> impl Fn() -> O
where
    O: Default,
{
    move || {
        if let Some(x) = x.upgrade() {
            f(x)
        } else {
            O::default()
        }
    }
}

pub fn bind_weak_fn1<T, A1, O>(x: Weak<T>, f: impl Fn(Rc<T>, A1) -> O) -> impl Fn(A1) -> O
where
    O: Default,
{
    move |a1| {
        if let Some(x) = x.upgrade() {
            f(x, a1)
        } else {
            O::default()
        }
    }
}

pub fn bind_weak_fnmut1<T, A1, O>(x: Weak<T>, mut f: impl FnMut(Rc<T>, A1) -> O) -> impl FnMut(A1) -> O
where
    O: Default,
{
    move |a1| {
        if let Some(x) = x.upgrade() {
            f(x, a1)
        } else {
            O::default()
        }
    }
}

pub fn bind_weak_async_fn0<T, O>(x: Weak<T>, f: impl AsyncFn(Rc<T>) -> O) -> impl AsyncFn() -> O
where
    O: Default,
{
    async move || {
        if let Some(x) = x.upgrade() {
            f(x).await
        } else {
            O::default()
        }
    }
}

pub fn bind_weak_async_fn1<T, A1, O>(
    x: Weak<T>,
    f: impl AsyncFn(Rc<T>, A1) -> O,
) -> impl AsyncFn(A1) -> O
where
    O: Default,
{
    async move |a1| {
        if let Some(x) = x.upgrade() {
            f(x, a1).await
        } else {
            O::default()
        }
    }
}

pub fn bind_weak_try_async_fn1<T, A1, O, E>(
    x: Weak<T>,
    f: impl AsyncFn(Rc<T>, A1) -> Result<O, E>,
) -> impl AsyncFn(A1) -> Result<O, E>
where
    O: Default,
{
    async move |a1| {
        if let Some(x) = x.upgrade() {
            f(x, a1).await
        } else {
            Ok(O::default())
        }
    }
}
