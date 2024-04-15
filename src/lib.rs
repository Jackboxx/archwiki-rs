#![warn(clippy::pedantic)]
#![allow(dead_code)]
#![allow(unused)]
#![allow(clippy::doc_markdown)]

#[cfg(all(feature = "cli", feature = "wasm-web"))]
compile_error!("the features 'cli' and 'wasm-web' can't be enabled at the same time!");

#[cfg(all(feature = "cli", feature = "wasm-nodejs"))]
compile_error!("the features 'cli' and 'wasm-nodejs' can't be enabled at the same time!");

#[cfg(all(test, not(feature = "cli")))]
compile_error!("tests have to be run with the 'cli' feature flag");

mod args;
mod error;
mod formats;
mod langs;
mod list;
mod search;
mod utils;
mod wiki;

#[cfg(feature = "cli")]
mod info;
#[cfg(feature = "cli")]
mod io;

#[cfg(any(feature = "wasm-nodejs", feature = "wasm-web"))]
mod wasm;

#[cfg(any(feature = "wasm-nodejs", feature = "wasm-web"))]
pub use wasm::*;