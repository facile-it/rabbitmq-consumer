fn main() {
    println!(r"cargo:rustc-link-lib=static-nobundle=mysqlclient");
    println!(r"cargo:rustc-link-lib=static-nobundle=stdc++");
    println!(r"cargo:rustc-link-lib=static-nobundle=z");
}
