#![feature(proc_macro)]

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate byteorder;
#[macro_use]
extern crate chan;
extern crate piston_window;
extern crate conrod;
extern crate nalgebra as na;
extern crate ncollide;

pub mod common;
pub mod client;
pub mod server;
