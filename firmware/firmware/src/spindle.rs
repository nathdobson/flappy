use crate::application::Application;
use crate::display::DisplayModule;
use crate::error::Error;
use core::alloc::AllocError;
use core::cell::RefCell;
use core::fmt::Display;
use core::pin::Pin;
use embassy_time::Delay;
use embedded_hal_async::delay;
use embedded_hal_async::delay::DelayNs;
use heapless::{String, Vec};
use log::error;
use protocol::display::DISPLAY_REQUEST_CAPACITY;
use spindle::compiler::ast::Program;
use spindle::compiler::codegen::Codegen;
use spindle::compiler::lexer::Lexer;
use spindle::compiler::parser::{AnnotatedParserError, Parser};
use spindle::compiler::stack::{Stack, StackBox, StackStorage, new_stack};
use spindle::interp::Interp;
use spindle::interp::heap::{Heap, HeapStorage};
use spindle::interp::value::Value;
use spindle::native::{NativeError, NativeFn, PrintFn};
use spindle::{Spindle, SpindleOptions};
use static_cell::StaticCell;
use ::make_static::make_static;

type MySpindle = Spindle<16384, 16384, 256, 256, 16384>;

struct SpindleState {
    spindle: &'static mut MySpindle,
}

pub struct SpindleModule {
    state: RefCell<SpindleState>,
}

impl SpindleModule {
    pub fn new() -> &'static Self {
        static SPINDLE: StaticCell<MySpindle> = StaticCell::new();
        let spindle = SPINDLE.init_with(|| Spindle::new());
        make_static!(
            SpindleModule,
            SpindleModule {
                state: RefCell::new(SpindleState { spindle }),
            }
        )
    }
    pub async fn run_program(
        &self,
        src: &str,
        #[cfg(feature = "display")] display: &'static DisplayModule,
    ) {
        self.state
            .borrow_mut()
            .run_program(
                src,
                #[cfg(feature = "display")]
                display,
            )
            .await;
    }
}

impl SpindleState {
    pub async fn run_program(
        &mut self,
        src: &str,
        #[cfg(feature = "display")] display: &'static DisplayModule,
    ) {
        if let Err(e) = self
            .spindle
            .run(
                SpindleOptions::default(),
                src,
                &[
                    &SleepMsFn,
                    &PrintFn,
                    &DisplayFn {
                        #[cfg(feature = "display")]
                        display: display,
                    },
                ],
            )
            .await
        {
            error!("Error running program: {:?}", e);
        }
    }
}

struct DisplayFn {
    #[cfg(feature = "display")]
    display: &'static DisplayModule,
}

impl NativeFn for DisplayFn {
    fn name(&self) -> &'static str {
        "display"
    }

    fn native_call<'call, 'stack, 'heap>(
        &'call self,
        stack: &'call mut Stack<'stack>,
        heap: &'call mut Heap<'heap>,
        args: &'call [Value],
    ) -> Result<
        Pin<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>>,
        AllocError,
    > {
        Ok(stack
            .push_init(async move {
                let mut text = String::<DISPLAY_REQUEST_CAPACITY>::new();
                for arg in args {
                    use core::fmt::Write;
                    match arg {
                        Value::Null => write!(text, "null").map_err(|_| NativeError)?,
                        Value::Bool(arg) => write!(text, "{}", arg).map_err(|_| NativeError)?,
                        Value::Number(arg) => write!(text, "{}", arg).map_err(|_| NativeError)?,
                        Value::Ref(x) => {
                            write!(text, "{}", heap.get(x)).map_err(|_| NativeError)?;
                        }
                    }
                }
                #[cfg(feature = "display")]
                self.display.display_once(&text).await;
                Ok(Value::Null)
            })?
            .into_pin())
    }
}

struct SleepMsFn;
impl NativeFn for SleepMsFn {
    fn name(&self) -> &'static str {
        "sleep_ms"
    }

    fn native_call<'call, 'stack, 'heap>(
        &'call self,
        stack: &'call mut Stack<'stack>,
        heap: &'call mut Heap<'heap>,
        args: &'call [Value],
    ) -> Result<
        Pin<StackBox<'call, dyn 'call + Future<Output = Result<Value, NativeError>>>>,
        AllocError,
    > {
        Ok(stack
            .push_init(async move {
                if let Some(arg) = args.get(0) {
                    match arg {
                        Value::Number(number) => {
                            Delay
                                .delay_ms((*number).try_into().ok().ok_or(NativeError)?)
                                .await;
                        }
                        _ => return Err(NativeError),
                    }
                } else {
                    return Err(NativeError);
                }
                Ok(Value::Null)
            })?
            .into_pin())
    }
}
