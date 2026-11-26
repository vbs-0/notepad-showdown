fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        // no_std pulls in none of these automatically
        println!("cargo:rustc-link-lib=c");
        println!("cargo:rustc-link-lib=dl");
        println!("cargo:rustc-link-lib=log");
        println!("cargo:rustc-link-lib=android");
    }
}
