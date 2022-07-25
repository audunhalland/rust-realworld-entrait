# rust-realworld-entrait
Taking inspiration from [realworld-axum-sqlx](https://github.com/launchbadge/realworld-axum-sqlx), here is a re-implementation of that app
using the [entrait pattern](https://docs.rs/entrait/latest/entrait/) to achieve loose coupling and flexible testability.

## Architecture
This application's architecture is deliberately a bit over-engineered, because it is mainly a showcase for Entrait.

It consists of several crates, demonstrating how to design abstractions. Crates are listed from most upstream to most downstream.

### `realworld_core`
This crate exports core data structures and abstractions, including:

* `RwError`: The error type used throughout the application.
* the `System` trait: Mockable system functions, like getting the current time
* the `GetConfig` trait: Abstraction over application configuration

### `realworld_db`
This crate encapsulates all database operations.

### `realworld_user`
This crate contains business logic for users and user authentication.

### `realworld_article`
This crate contains the business logic for articles.

### `realworld_app`
This crate consists of a library to wire thing together into a running backend application, and a small `main` crate that produces a running executable.

It contains the central `App` data structure and Axum HTTP handlers.
