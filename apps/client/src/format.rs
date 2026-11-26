//! Small presentation helpers.

/// Formats an RFC 3339 timestamp as `26 Nov 2026`.
///
/// The API always sends ISO 8601 strings, so a full date parser is not needed
/// for this milestone; anything unexpected falls back to the raw value.
pub fn short_date(value: &str) -> String {
    let date = value.get(..10).unwrap_or(value);
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return date.to_owned();
    };

    let month = match month {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => return date.to_owned(),
    };

    format!("{day} {month} {year}")
}
