pub mod simple_engine;

#[cfg(feature = "fastembed-engine")]
pub mod fastembed_engine;

#[cfg(feature = "fastembed-engine")]
pub use fastembed_engine::FastEmbedEngine;
pub use simple_engine::SimpleEmbedEngine;
