fn main() {
    println!(r"cargo:rustc-link-lib=static=mysqlclient");
    println!(r"cargo:rustc-link-lib=static=stdc++");
    println!(r"cargo:rustc-link-lib=static=z");
}
