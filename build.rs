use std::env;

fn main() {
    if "release" == env::var("PROFILE").unwrap_or_else(|_| "".into()) {
        println!(r"cargo:rustc-link-lib=static=mysqlclient");
        println!(r"cargo:rustc-link-lib=static:-bundle=stdc++");
        println!(r"cargo:rustc-link-lib=static=z");
        println!(r"cargo:rustc-link-lib=static=ssl");
        println!(r"cargo:rustc-link-lib=static=crypto");
    }
}
