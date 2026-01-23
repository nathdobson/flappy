#[macro_export]
macro_rules! make_static {
    ($t:ty, $e:expr) => {
        {
            static STATIC: ::static_cell::StaticCell<$t> = ::static_cell::StaticCell::new();
            STATIC.init($e)
        }
    };
}
