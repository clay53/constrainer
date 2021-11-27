use constrained::create_constrainer;

fn compute_y(x: f32) -> f32 {
    x*10.0-1.0
}

create_constrainer!(MyConstrainer {
    dynamic x f32
    constrained y f32 (dynamic x f32) {
        compute_y(x)
    }
    constrained z f32 (dynamic x f32, constrained y f32) {
        x*y
    }
});

fn main() {
    let constrainer_instance = MyConstrainer::new(2.0);
    let y = constrainer_instance.get_y();
    let z = constrainer_instance.get_z();
    assert_eq!(y, compute_y(2.0));
    assert_eq!(z, y*2.0);
}