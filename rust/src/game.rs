// Re-expose the same `game.rs` the browser inlines, so user code compiled here
// behaves identically to the browser build. `game::run` is only used when the
// user compiles their own binary against stdin/stdout; the native simulator
// drives `reducer::tick` directly and never calls `run`.

#[path = "../../public/rust/game.rs"]
mod inner;

pub use inner::*;
