#![no_std]

#[doc(hidden)]
pub mod reexports {
    pub use ::static_cell;
}

#[macro_export]
macro_rules! make_static {
    ($t:ty, $e:expr) => {
        {
            static STATIC: $crate::reexports::static_cell::StaticCell<$t> = $crate::reexports::static_cell::StaticCell::new();
            STATIC.init($e)
        }
    };
    ($e:expr) => {
        {
            type T = impl ::core::marker::Sized;
            static STATIC: $crate::reexports::static_cell::StaticCell<T> = $crate::reexports::static_cell::StaticCell::new();
            #[deny(unused_attributes)]
            let (x,) = STATIC.uninit().write(($e,));
            x
        }
    }
}
