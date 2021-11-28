[![Rust](https://github.com/clay53/constrained/actions/workflows/rust.yml/badge.svg)](https://github.com/clay53/constrained/actions/workflows/rust.yml)
# Constrainer
An attempt to bring CAD concepts of constraints to variables in order to significantly reduce redundant instructions.

## Usage
create_constrainer! creates a struct that serves as the basis for the "constraining" environment for your constrained variables. The first word (Ident) is the name of your constrainer struct. This can be any valid struct name. Right after, place braces to deliminate the data that will passed to the compiler (proc_macro2) to create your constrainer struct. Inside of the braces, you can define dynamics & constrained variables, and operations those variables can undergo.

Dynamics are defined as follows: `dynamic name type`

Constraineds are defined as follows: `constrained name type (args) { set fn body }`

Variables can be retrieved by calling `.get_{name}` on an instance of your constrainer;

Constraineds can also depend on other constraineds. Ensure that this is done in the order of dependencies. All operations will, by default, be performed linearly in the order they were defined.

Listeners are defined as follows: `listener name (args) { listener fn body }`. Listeners are called when the variables in its arguments are updated or initialized.

Note: Does not currently support referencing types outside of the current scope `std::f32` will *not* work. This feature may be implemented in a future version.

Note2: Commas are currently ignored. Do not depend on this. They will become mandatory in a future version.

Note3: Really not done.

create_constrainer! example:
```rust
use constrained::create_constrainer;

fn compute_y(x: f32) -> f32 {
    x*10.0-1.0
}

create_constrainer!(MyConstrainer {
    dynamic x f32
    constrained y f32 (x) {
        compute_y(x)
    }
    constrained z f32 (x, y) {
        x*y
    }
});

fn main() {
    let constrainer_instance = MyConstrainer::new(2.0);
    let y = constrainer_instance.get_y();
    let z = constrainer_instance.get_z();
    assert_eq!(*y, compute_y(2.0));
    assert_eq!(*z, y*2.0);
}
```
