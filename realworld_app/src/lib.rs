#![cfg_attr(feature = "use-associated-future", feature(generic_associated_types))]
#![cfg_attr(feature = "use-associated-future", feature(type_alias_impl_trait))]

pub mod app;
pub mod config;
pub mod routes;

mod article;
mod auth;
mod password;
mod profile;
mod user;

#[cfg(test)]
mod test_util;
