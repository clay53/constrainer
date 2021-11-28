use constrained::create_constrainer;

create_constrainer!(Constrainer {
    dynamic x f32
    dynamic y f32
    external ex String
    
    constrained xy f32 (x, y) {
        println!("Computing xy");
        x*y
    }
    constrained xy_x f32 (xy, x) {
        println!("Computing xy_x");
        xy*x
    }
    constrained xy_x2 f32 (xy_x) {
        println!("Computing xy_x2");
        xy_x*2.0
    }
    constrained x2 f32 (x) {
        println!("Computing x2");
        x*2.0
    }
    constrained x2sin f32 (x2) {
        println!("Computing x2sin");
        x2.sin()
    }
    constrained y_ex f32 (y, ex) {
        println!("Computer yex! {}", ex);
        y
    }
    
    opgenset (y, x)
    opgenset (x)
    opgenset (y)
});

fn main() {
    println!("Initializing");
    let mut constrainer = Constrainer::new(3.0, 2.0, "Initial y_ex".to_string());
    println!("{:?}", constrainer);

    println!("\nSetting x & y");
    constrainer.set_x_y(2.0, 4.0, "y_ex when setting x & y".to_string());
    println!("{:?}", constrainer);

    println!("\nSetting x");
    constrainer.set_x(9.0);
    println!("{:?}", constrainer);

    println!("\nSetting y");
    constrainer.set_y(6.0, "y_ex when setting y".to_string());
    println!("{:?}", constrainer);
}