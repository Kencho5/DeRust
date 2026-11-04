use std::time::Duration;

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

pub fn duration(duration: Duration) -> String {
    let millis = duration.as_millis();

    match millis {
        0..1_000 => format!("{millis} ms"),
        1_000..60_000 => format!("{:.1} s", duration.as_secs_f64()),
        _ => format!("{} m {} s", millis / 60_000, millis % 60_000 / 1_000),
    }
}

pub fn count(count: u64) -> String {
    let digits = count.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }

    grouped
}
