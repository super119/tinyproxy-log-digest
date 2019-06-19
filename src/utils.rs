use crate::errors::*;

fn is_leap_year(y: u32) -> bool {
    if (y % 4 == 0 && y % 100 != 0) ||
        (y % 400 == 0 && y % 3200 != 0) {
        true
    } else {
        false
    }
}

// Convert human readable date time to UNIX timestamp
// E.g: Jun 13 12:16:47 2019
pub fn date_time_to_timestamp(dt: String) -> Result<u64> {
    if dt.trim().len() == 0 {
        return Ok(0);
    }
    let mut toks: Vec<String> = Vec::new();
    let splits = dt.split(' ');
    for s in splits {
        if s.trim().len() > 0 {
            toks.push(s.to_string());
        }
    }

    let mut result: u64 = 0;
    let year: u32 = toks[3].parse()?;
    let month: u32;
    match toks[0].as_str() {
        "Jan" => month = 0,
        "Feb" => month = 1,
        "Mar" => month = 2,
        "Apr" => month = 3,
        "May" => month = 4,
        "Jun" => month = 5,
        "Jul" => month = 6,
        "Aug" => month = 7,
        "Sep" => month = 8,
        "Oct" => month = 9,
        "Nov" => month = 10,
        "Dec" => month = 11,
        _ => bail!(ErrorKind::InvalidMonth)
    }
    let day: u32 = toks[1].parse()?;
    let time_toks: Vec<&str> = toks[2].split(':').map(|x| x.trim()).collect();
    let hour: u32 = time_toks[0].parse()?;
    let minute: u32 = time_toks[1].parse()?;
    let second: u32 = time_toks[2].parse()?;

    let mut y = 1970;
    while y < year {
        if is_leap_year(y) {
            result += 3600 * 24 * 366;
        } else {
            result += 3600 * 24 * 365;
        }
        y += 1;
    }
    let mut m = 0;
    while m < month {
        match m {
            0 | 2 | 4 | 6 | 7 | 9 | 11 => result += 3600 * 24 * 31,
            3 | 5 | 8 | 10 => result += 3600 * 24 * 30,
            1 => {
                if is_leap_year(year) {
                    result += 3600 * 24 * 29;
                } else {
                    result += 3600 * 24 * 28;
                }
            },
            _ => unreachable!()
        }
        m += 1;
    }
    result += 3600 * 24 * ((day - 1) as u64);
    result += 3600 * (hour as u64) + 60 * (minute as u64) + (second as u64);

    Ok(result)
}

pub fn timestamp_to_date_time(mut ts: u64) -> String {
    let mut year = 0;
    let month;
    let day;
    let hour;
    let minute;
    let second;

    let mut m = 0;
    let mut elapsed: u64 = 0;
    let mut elapsed_last: u64 = 0;;
    loop {
        match m % 12 {
            0 | 2 | 4 | 6 | 7 | 9 | 11 => elapsed += 3600 * 24 * 31,
            3 | 5 | 8 | 10 => elapsed += 3600 * 24 * 30,
            1 => {
                if is_leap_year(1970 + year) {
                    elapsed += 3600 * 24 * 29;
                } else {
                    elapsed += 3600 * 24 * 28;
                }
            },
            _ => unreachable!()
        }

        if elapsed > ts {
            month = m % 12;
            ts -= elapsed_last;
            break;
        }
        m += 1;
        year = m / 12;
        elapsed_last = elapsed;
    }

    day = ts / (3600 * 24);
    ts %= 3600 * 24;
    hour = ts / 3600;
    ts %= 3600;
    minute = ts / 60;
    ts %= 60;
    second = ts;

    let result: String;
    let month_string: String;
    match month {
        0 => month_string = String::from("Jan"),
        1 => month_string = String::from("Feb"),
        2 => month_string = String::from("Mar"),
        3 => month_string = String::from("Apr"),
        4 => month_string = String::from("May"),
        5 => month_string = String::from("Jun"),
        6 => month_string = String::from("Jul"),
        7 => month_string = String::from("Aug"),
        8 => month_string = String::from("Sep"),
        9 => month_string = String::from("Oct"),
        10 => month_string = String::from("Nov"),
        11 => month_string = String::from("Dec"),
        _ => unreachable!()
    }
    let hour_string: String;
    let minute_string: String;
    let second_string: String;
    if hour < 10 {
        hour_string = format!("0{}", hour);
    } else {
        hour_string = format!("{}", hour);
    }
    if minute < 10 {
        minute_string = format!("0{}", minute);
    } else {
        minute_string = format!("{}", minute);
    }
    if second < 10 {
        second_string = format!("0{}", second);
    } else {
        second_string = format!("{}", second);
    }

    result = format!("{} {} {}:{}:{} {}", month_string, day + 1,
                hour_string, minute_string, second_string, 1970 + year);
    result
}

pub fn timestamp_get_year(ts: u64) -> u32 {
    let mut year = 0;
    let mut m = 0;
    let mut elapsed: u64 = 0;
    loop {
        match m % 12 {
            0 | 2 | 4 | 6 | 7 | 9 | 11 => elapsed += 3600 * 24 * 31,
            3 | 5 | 8 | 10 => elapsed += 3600 * 24 * 30,
            1 => {
                if is_leap_year(1970 + year) {
                    elapsed += 3600 * 24 * 29;
                } else {
                    elapsed += 3600 * 24 * 28;
                }
            },
            _ => unreachable!()
        }

        if elapsed > ts {
            break;
        }
        m += 1;
        year = m / 12;
    }

    (1970 + year)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_time_to_timestamp() {
        let result = date_time_to_timestamp(String::from("Jan 1 00:00:00 1970")).unwrap();
        assert_eq!(result, 0);
        let result = date_time_to_timestamp(String::from("Jun 14 20:25:00 2019")).unwrap();
        assert_eq!(result, 1560543900);
        let result = date_time_to_timestamp(String::from("Oct 29 12:31:56 2020")).unwrap();
        assert_eq!(result, 1603974716);
    }

    #[test]
    fn test_timestamp_to_date_time() {
        let result = timestamp_to_date_time(0);
        assert_eq!(result, String::from("Jan 1 00:00:00 1970"));
        let result = timestamp_to_date_time(1560543900);
        assert_eq!(result, String::from("Jun 14 20:25:00 2019"));
        let result = timestamp_to_date_time(1603974716);
        assert_eq!(result, String::from("Oct 29 12:31:56 2020"));
    }

    #[test]
    fn test_timestamp_get_year() {
        let result = timestamp_get_year(1560543900);
        assert_eq!(result, 2019);
        let result = timestamp_get_year(1603974716);
        assert_eq!(result, 2020);
    }
}
