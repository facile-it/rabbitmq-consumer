fn main() {
    println!(r"cargo:rustc-link-lib=static=mysqlclient");
    println!(r"cargo:rustc-link-lib=static-nobundle=stdc++");
    println!(r"cargo:rustc-link-lib=static=z");
    println!(r"cargo:rustc-link-lib=static=ssl");
    println!(r"cargo:rustc-link-lib=static=crypto");
}
