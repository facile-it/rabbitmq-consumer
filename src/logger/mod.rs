use chrono::Local;

pub fn log<S: AsRef<str>>(message: S) {
    if !cfg!(test) {
        println!(
            "{} - {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            message.as_ref()
        );
    }
}
