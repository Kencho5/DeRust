pub fn bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];

    if bytes < 1000 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit = "B";

    for next in UNITS {
        if value < 1000.0 {
            break;
        }
        value /= 1000.0;
        unit = next;
    }

    format!("{value:.1} {unit}")
}
