use crate::application::Application;
use crate::controller::ControllerModule;
use crate::display::DisplayModule;
use crate::error::Error;
use crate::make_static;
use arena::Arena;
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
use spindle::Spindle;
use spindle::compiler::ast::Program;
use spindle::compiler::codegen::Codegen;
use spindle::compiler::lexer::Lexer;
use spindle::compiler::parser::{AnnotatedParserError, Parser};
use spindle::compiler::stack::{Stack, StackBox, StackStorage, new_stack};
use spindle::interp::Interp;
use spindle::interp::heap::{Heap, HeapStorage};
use spindle::interp::value::Value;
use spindle::native::{NativeError, NativeFn, PrintFn};

struct SpindleState {
    spindle: Spindle<65536, 65536, 256, 256, 65536>,
}

pub struct SpindleModule {
    state: RefCell<SpindleState>,
}

impl SpindleModule {
    pub fn new() -> &'static Self {
        make_static!(
            SpindleModule,
            SpindleModule {
                state: RefCell::new(SpindleState {
                    spindle: Spindle::new()
                }),
            }
        )
    }
    pub async fn run_program(&self, src: &str, display: &'static DisplayModule) {
        self.state.borrow_mut().run_program(src, display).await;
    }
}

impl SpindleState {
    pub async fn run_program(&self, src: &str, display: &'static DisplayModule) {

        // let mut arena = [0; 65536];
        // let stack =
        // let Ok(arena) = Arena::new(&mut arena) else {
        //     error!("Can't start arena");
        //     return;
        // };
        // let lexer = Lexer::new(src, arena);
        // let mut parser = Parser::new(lexer, arena);
        // let program = match parser.parse_program() {
        //     Ok(program) => program,
        //     Err(e) => {
        //         error!("Parse failed: {:?}", e);
        //         return;
        //     }
        // };
        // let natives: &[&dyn NativeFn] = &[&PrintFn, &DisplayFn { display }, &SleepMsFn];
        // let mut compiler = Codegen::new(arena, natives, &program);
        // let program = match compiler.compile() {
        //     Ok(program) => program,
        //     Err(e) => {
        //         error!("Compilation failed: {:?}", e);
        //         return;
        //     }
        // };
        // let mut value_stack = Vec::<_, 128>::new();
        // let mut heap_storage = HeapStorage::<128, 65536>::new();
        // let mut interp = Interp::new(&program, &mut value_stack, heap_storage.start(), natives);
        // let mut stack = new_stack::<65536>();
        // let mut stack: &mut StackStorage = &mut stack;
        // match interp.interp(stack.start()).await {
        //     Ok(_) => {}
        //     Err(e) => {
        //         error!("Execution failed: {:?}", e);
        //         return;
        //     }
        // };
    }
}

struct DisplayFn {
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
