use crate::error::Error;
use embassy_executor::Spawner;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::Builder;

pub trait Buffer: Sized + AsMut<[u8]> {
    fn zeroed() -> Self;
}
impl<const N: usize> Buffer for [u8; N] {
    fn zeroed() -> Self {
        [0; N]
    }
}

pub trait UsbServer: 'static {
    type ConfigDescBuffer: Buffer;
    type BosDescBuffer: Buffer;
    type MsosDescBuffer: Buffer;
    fn build(
        &'static self,
        spawner: Spawner,
        builder: &mut Builder<'static, embassy_rp::usb::Driver<'static, USB>>,
    ) -> Result<(), Error>;
}

impl<T: UsbServer> UsbServer for &'static T {
    type ConfigDescBuffer = T::ConfigDescBuffer;
    type BosDescBuffer = T::BosDescBuffer;
    type MsosDescBuffer = T::MsosDescBuffer;

    fn build(
        &'static self,
        spawner: Spawner,
        builder: &mut Builder<'static, Driver<'static, USB>>,
    ) -> Result<(), Error> {
        (**self).build(spawner, builder)
    }
}

#[macro_export]
macro_rules! UsbServer {
    derive() (
        $vis:vis struct $name:ident {
            $(
                $field:ident: $typ:ty,
            )*
        }
    ) => {
        impl $crate::UsbServer for $name {
            type ConfigDescBuffer =
                [u8;
                    $(
                        (size_of::<<$typ as $crate::UsbServer>::ConfigDescBuffer>()) +
                    )*0
                ];
            type BosDescBuffer =
                [u8;
                    $(
                        (size_of::<<$typ as $crate::UsbServer>::BosDescBuffer>()) +
                    )*0
                ];
            type MsosDescBuffer =
                [u8;
                    $(
                        (size_of::<<$typ as $crate::UsbServer>::MsosDescBuffer>()) +
                    )*0
                ];
            fn build(
                &'static self,
                spawner: $crate::reexports::embassy_executor::Spawner,
                builder: &mut $crate::reexports::embassy_usb::Builder<'static, $crate::reexports::embassy_rp::usb::Driver<'static, $crate::reexports::embassy_rp::peripherals::USB>>,
            ) ->Result<(), $crate::error::Error>{
                $(
                    self.$field.build(spawner, builder)?;
                )*
                Ok(())
            }
        }

    }
}
