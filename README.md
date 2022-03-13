# rust-realworld-ioc
Taking inspiration from [realworld-axum-sqlx](https://github.com/launchbadge/realworld-axum-sqlx), here is an attempt to implement
the same application using _Inversion of Control_ (IoC) patterns that enables flexible unit testing.

The project uses a new and experimental _IoC_ library called [entrait](https://docs.rs/entrait/latest/entrait/). The experiment tries to
establish a simple application architecture that is as close to _zero-cost_ as possible and keeping the amount of boilerplate code
to an absolute minimum. This is easier said than done in a language that lacks reflection, but quite fun to try to implement.

_Zero-cost Inversion of Control_ is easily achieved in Rust using traits and generics. A current notable exception to this rule is `async` functions, which need
the `async_trait` hack, which puts futures in a `Box`. Zero-cost _async functions in traits_ are currently on the Rust roadmap.

In the meantime, enjoy your _Enterprise Rust_!
