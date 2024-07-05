fn main() {
    println!("Run this with ASAN enabled, it will detect a memleak");

    let b = Box::new(123);
    Box::leak(b);
}
