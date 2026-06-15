#[cfg(feature = "jit")]
use elara::{JitMode, Lua};

#[cfg(feature = "jit")]
fn main() {
    let lua = Lua::builder().jit(JitMode::Always).build();

    let values = lua
        .eval("local total = 0\nfor i = 1, 10 do total = total + i end\nreturn total")
        .expect("chunk should evaluate");
    assert_eq!(
        values.first().and_then(|value| value.as_integer()),
        Some(55)
    );
}

#[cfg(not(feature = "jit"))]
fn main() {
    eprintln!("run with `cargo run -p elara --features jit --example jit_embed`");
}
