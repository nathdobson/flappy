use crate::compiler::stack::{Stack, StackBox};
use core::alloc::{AllocError, Layout};
use core::cell::RefCell;
use core::future::poll_fn;
use core::marker::PhantomData;
use core::mem;
use core::pin::{Pin, UnsafePinned, pin};
use core::ptr::{NonNull, null_mut};
use core::task::Poll;

struct StackTask<F: ?Sized = dyn 'static + Future<Output = ()>> {
    previous: Option<NonNull<StackTask>>,
    future: F,
}

struct StackState {
    top_task: Option<NonNull<StackTask>>,
    repoll: bool,
}

struct StackExecutor {
    state: RefCell<StackState>,
}

pub struct StackSpawn<'a> {
    executor: &'a StackExecutor,
    alloc: Stack<'a>,
}

type ReturnByRef<'a, F: 'a + Future> = impl 'a + Future<Output = ()>;

#[define_opaque(ReturnByRef)]
fn return_by_ref<'a, F: 'a + Future>(
    f: F,
    output: &'a mut Option<F::Output>,
) -> ReturnByRef<'a, F> {
    async move { *output = Some(f.await) }
}

async fn spawn<'s, O, F: for<'b> AsyncFnOnce(StackSpawn<'b>) -> O>(
    executor: &'_ StackExecutor,
    mut alloc: Stack<'s>,
    task: F,
    cb: impl Future,
) -> Result<O, AllocError> {
    unsafe {
        let previous = executor.state.borrow().top_task;
        let layout = Layout::new::<
            StackTask<ReturnByRef<<F as AsyncFnOnce<(StackSpawn<'static>,)>>::CallOnceFuture>>,
        >();
        let (alloc, slot) = alloc.push(layout)?;
        let mut output = None;
        let mut task_box: StackBox<
            StackTask<ReturnByRef<<F as AsyncFnOnce<(StackSpawn<'_>,)>>::CallOnceFuture>>,
        > = slot.init(StackTask {
            previous,
            future: return_by_ref(task(StackSpawn { executor, alloc }), &mut output),
        })?;
        let task: &mut StackTask<dyn '_ + Future<Output = ()>> = &mut *task_box;
        let task: NonNull<StackTask<dyn '_ + Future<Output = ()>>> = NonNull::from_mut(task);
        let task: NonNull<StackTask<dyn 'static + Future<Output = ()>>> = mem::transmute(task);
        *executor.state.borrow_mut() = StackState {
            top_task: Some(task),
            repoll: true,
        };
        cb.await;
        mem::drop(task_box);
        Ok(output.unwrap())
    }
}

pub async fn stack_executor<O, F: for<'a> AsyncFnOnce(StackSpawn<'a>) -> O>(
    alloc: Stack<'_>,
    f: F,
) -> Result<O, AllocError> {
    let executor = StackExecutor {
        state: RefCell::new(StackState {
            top_task: None,
            repoll: false,
        }),
    };
    let output = spawn(
        &executor,
        alloc,
        f,
        poll_fn(|cx| unsafe {
            loop {
                let top_task = executor.state.borrow().top_task;
                if let Some(mut top_task) = top_task {
                    match Pin::new_unchecked(&mut (top_task.as_mut()).future).poll(cx) {
                        Poll::Ready(()) => {
                            executor.state.borrow_mut().top_task = top_task.as_mut().previous;
                            continue;
                        }
                        Poll::Pending => {
                            if mem::replace(&mut executor.state.borrow_mut().repoll, false) {
                                continue;
                            } else {
                                return Poll::Pending;
                            }
                        }
                    }
                } else {
                    return Poll::Ready(());
                }
            }
        }),
    )
    .await?;
    Ok(output)
}

impl<'a> StackSpawn<'a> {
    pub fn reborrow(&mut self) -> StackSpawn<'_> {
        StackSpawn {
            executor: self.executor,
            alloc: self.alloc.reborrow(),
        }
    }
    pub async fn recurse<O, F: for<'b> AsyncFnOnce(StackSpawn<'b>) -> O>(
        self,
        f: F,
    ) -> Result<O, AllocError> {
        let mut polled = false;
        Ok(spawn(
            self.executor,
            self.alloc,
            f,
            poll_fn(|cx| {
                if mem::replace(&mut polled, true) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }),
        )
        .await?)
    }
}
