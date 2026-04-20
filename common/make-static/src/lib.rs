#![no_std]

#[doc(hidden)]
pub mod reexports{
    pub use ::static_cell as static_cell;
}

#[macro_export]
macro_rules! make_static {
    ($t:ty, $e:expr) => {
        {
            static STATIC: $crate::reexports::static_cell::StaticCell<$t> = $crate::reexports::static_cell::StaticCell::new();
            STATIC.init($e)
        }
    };
}
