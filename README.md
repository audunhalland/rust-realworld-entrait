# rust-realworld-entrait
Taking inspiration from [realworld-axum-sqlx](https://github.com/launchbadge/realworld-axum-sqlx), here is a re-implementation of that app
using the [entrait pattern](https://docs.rs/entrait/latest/entrait/) to achieve loose coupling and flexible testability.

## 🧅 Architecture
This application's architecture is deliberately a bit over-engineered, because it is mainly a showcase for Entrait.

It uses an _onion architecture_, with different layers modelled as crates. The crates are described here, from innermost to outermost:

### `realworld_domain`
This crate contains domain types, core business logic and various abstraction layers (i.e. traits).

It is not a runnable application on its own, but exports business logic as _traits_ that can be trivially implemented for any application.

The crate is abstract over any kind of external systems the resulting application needs to be dependent upon,
    like database/storage, application environment (like clock) and specific configuration parameters.
All such abstractions are defined as traits that must be implemented outside this crate.

Example of potentially interesting code in this crate is
    the [user module](realworld_domain/src/user/mod.rs),
    its [repository abstraction](realworld_domain/src/user/repo.rs),
    or the [system abstractions](realworld_domain/src/lib.rs).

### `realworld_db`
This crate implements _repository_ traits from `realworld_domain`, and re-exports those for use by an application.

[This is how](realworld_db/src/user.rs) the user repository implementation looks like.

### `realworld_app`
This crate contains the [main function](realworld_app/src/main.rs) and compiles into an executable binary.

It contains the central [`App` data structure](realworld_app/src/app.rs) that is used with entrait, and [Axum HTTP handlers](realworld_app/src/routes/mod.rs).

The `App` implements various traits from `realworld_domain` to make them work together.

The crate contains various [unit tests](realworld_app/src/routes/user_routes.rs) for HTTP handlers. Yes, pure unit tests!
