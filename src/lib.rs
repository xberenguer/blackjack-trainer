#![no_std]
#![forbid(unsafe_code)]
#[cfg(test)]
extern crate std;

/// Version of this exact Rust game core, shared by device and browser metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cards;
pub mod controls;
pub mod counting;
pub mod deviations;
pub mod engine;
pub mod rules;
pub mod strategy;

pub mod assets;
pub mod canvas;
pub mod round;
pub mod render;

pub mod audio;

pub mod guide;
pub mod table;
pub mod text;

pub mod training;

pub mod count_practice;
pub mod persistence;

pub mod ev;
pub mod split_ev;

pub mod app;
mod screens;
pub mod widgets;

pub mod wager_practice;

pub mod seats;

pub mod deviation_drills;

#[doc(hidden)]
pub mod resume_binding;
#[doc(hidden)]
pub mod resume_private;
pub mod resume_ui;

pub mod session_analysis;
pub mod session_metrics;

pub mod audio_gain;
