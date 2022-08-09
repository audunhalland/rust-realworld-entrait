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

### `realworld_db`
This crate implements _repository_ traits from `realworld_domain`, and re-exports those for use by an application.

### `realworld_app`
This crate consists of a library to wire things together into a running backend application, and a small `main` crate that produces a running executable.

It contains the central `App` data structure and Axum HTTP handlers.

The `App` implements various traits from `realworld_domain` to make them work together.
