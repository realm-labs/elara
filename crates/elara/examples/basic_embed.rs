use elara::{Lua, NativeFunctionError};

fn main() {
    let lua = Lua::new();

    let values = lua
        .eval("local answer = 40 + 2\nreturn answer")
        .expect("chunk should evaluate");
    assert_eq!(
        values.first().and_then(|value| value.as_integer()),
        Some(42)
    );

    let add = lua.create_function(|(left, right): (i64, i64)| {
        Ok::<(i64,), NativeFunctionError>((left + right,))
    });
    lua.set_global_function("add", add);

    let values = lua
        .eval("return add(20, 22)")
        .expect("chunk should evaluate");
    assert_eq!(
        values.first().and_then(|value| value.as_integer()),
        Some(42)
    );
}
