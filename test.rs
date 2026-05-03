fn main() {
    let t = 40.0f32.to_bits();
    let l = 85.0f32.to_bits();
    println!("t = {}, l = {}, t>=l is {}", t, l, t >= l);
}
