
pub const SERIAL_MTU: usize = 10;

#[cfg(feature = "uuid")]
pub const FLAPPY_SERVICE_UUID: uuid::Uuid = uuid::uuid!("5af0b930-b9b5-11f0-b558-0800200c9a66");
#[cfg(feature = "uuid")]
pub const SERIAL_OUT_UUID: uuid::Uuid = uuid::uuid!("2d2bc907-c9fa-49fd-ba45-410cddf61e5c");
#[cfg(feature = "uuid")]
pub const SERIAL_IN_UUID: uuid::Uuid = uuid::uuid!("4574529b-fbe4-44ae-ba52-d877ac76ef2d");
#[cfg(feature = "uuid")]
pub const APP_STATUS_UUID: uuid::Uuid = uuid::uuid!("4dc5669d-6bc8-40eb-b6af-8091d4e9b713");

#[cfg(feature = "uuid")]
pub const DUMMY_UUID: uuid::Uuid = uuid::uuid!("4a02f134-cc41-4a7b-94af-e29d290cffa5");
