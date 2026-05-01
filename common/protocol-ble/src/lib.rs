#![no_std]
pub const SERIAL_MTU: usize = 64;

macro_rules! define_uuids {
    ($ty: ty) => {
        pub const RPC_SERVICE_UUID: $ty = uuid!("5af0b930-b9b5-11f0-b558-0800200c9a66");
        pub const SERIAL_OUT_UUID: $ty = uuid!("2d2bc907-c9fa-49fd-ba45-410cddf61e5c");
        pub const SERIAL_IN_UUID: $ty = uuid!("4574529b-fbe4-44ae-ba52-d877ac76ef2d");
        pub const APP_STATUS_UUID: $ty = uuid!("4dc5669d-6bc8-40eb-b6af-8091d4e9b713");
    };
}

#[cfg(feature = "uuid")]
pub mod uuid {
    use uuid::uuid;
    define_uuids!(uuid::Uuid);
}

#[cfg(feature = "trouble-host")]
pub mod trouble_host {
    use trouble_host::prelude::uuid;
    define_uuids!(trouble_host::types::uuid::Uuid);
}
