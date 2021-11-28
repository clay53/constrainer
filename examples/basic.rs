use constrained::create_constrainer;

type StaticStr = &'static str;
create_constrainer!(Constrainer {
    dynamic x f32
    dynamic y f32
    dynamic z f32
    external ex StaticStr
    external ex2 f32
    
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
        println!("Computing yex! {}", ex);
        y
    }
    listener z_reporter (z, ex) {
        println!("z was updated to {}. Also, I hear ex was: {}", z, ex);
    }
    listener z_reporter2 (z, ex2) {
        println!("z was updated to {}. Also, I hear ex2 was: {}", z, ex2);
    }
    
    opgenset (y, x)
    opgenset (x)
    opgenset (y)
    opgenset (z)
});

fn main() {
    println!("Initializing");
    let mut constrainer = Constrainer::new(3.0, 2.0, 7.0, "Initial y_ex", 5.0);
    println!("{:?}", constrainer);

    println!("\nSetting x & y");
    constrainer.set_x_y(2.0, 4.0, "y_ex when setting x & y");
    println!("{:?}", constrainer);

    println!("\nSetting x");
    constrainer.set_x(9.0);
    println!("{:?}", constrainer);

    println!("\nSetting y");
    constrainer.set_y(6.0, "y_ex when setting y");
    println!("{:?}", constrainer);

    println!("\nSetting z");
    constrainer.set_z(11.0, "y_ex when setting z", 99.0);
    println!("{:?}", constrainer);
}