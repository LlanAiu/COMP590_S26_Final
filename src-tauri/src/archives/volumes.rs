// builtin
use std::future::Future;

// external

// internal

// modules
pub mod implementations;
pub mod subsystems;
pub mod types;

pub trait VolumeDatabase {
    fn create_volume(&self) -> impl Future<Output = ()> + Send;

    fn read_volume(&self) -> impl Future<Output = ()> + Send;

    fn edit_volume(&self) -> impl Future<Output = ()> + Send;

    fn delete_volume(&self) -> impl Future<Output = ()> + Send;
}
