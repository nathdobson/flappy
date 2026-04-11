use futures_util::future::LocalBoxFuture;
use std::env::Args;
use std::marker::Tuple;
use std::rc::Rc;

pub trait DynAsyncFn<Args: Tuple> {
    type Output;
    fn async_call_dyn<'a>(&'a self, args: Args) -> LocalBoxFuture<'a, Self::Output>;
}

impl<T, Args: 'static + Tuple> DynAsyncFn<Args> for T
where
    T: AsyncFn<Args>,
{
    type Output = T::Output;

    fn async_call_dyn<'a>(&'a self, args: Args) -> LocalBoxFuture<'a, Self::Output> {
        Box::pin(self.async_call(args))
    }
}

impl<O> dyn DynAsyncFn<(), Output = O> {
    pub async fn call(&self) -> O {
        self.async_call_dyn(()).await
    }
}

impl<O, A1> dyn DynAsyncFn<(A1,), Output = O> {
    pub async fn call(&self, a1: A1) -> O {
        self.async_call_dyn((a1,)).await
    }
}
