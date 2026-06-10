//! Core routing engine for BrowserRouter (`br`).
//!
//! This crate is platform-independent: it owns the configuration model, URL
//! filters (tracking-param stripping, HTTPS upgrade, regex rewrite) and the
//! rule-matching engine that decides where a URL should be opened.

pub mod config;
pub mod engine;
pub mod filters;
pub mod model;

pub use config::{load_or_default, parse, validate};
pub use engine::{explain, route};
pub use model::{
    Action, BrowserTarget, Config, DefaultAction, Filter, General, MatchCondition, Rule,
    RoutingContext, RoutingDecision, RoutingExplanation,
};
