//! Core domain types and ports for Project Verity.

pub mod agent;
pub mod arena;
pub mod environment;
pub mod game;
pub mod games;
pub mod llm;
pub mod session;
pub mod tool;

#[cfg(test)]
mod test_support;
