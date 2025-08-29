fn main() {
    cc::Build::new()
        .file("src/utils/random.c")
        .compile("random");

    println!("cargo:reran-if-changed=src/utils/random.c");
    println!("cargo:reran-if-changed=src/utils/random.h");
}