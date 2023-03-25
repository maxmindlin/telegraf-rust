/// Shorthand macro for generating
/// [crate::Point] structs.
///
/// Syntax is a measurement, followed by
/// (optional) space-delineated tag tuples, followed by
/// space-delineated field tuples, followed by
/// (optional) a semicolon and a timestamp.
///
/// Every tuple member except field values must be &str. Field values
/// must implement [crate::IntoFieldData].
///
/// `(<measurement>, [(<tagName>, <tagVal>)], [(<fieldName>, <fieldVal>)]; <timestamp>)`
///
/// Influx protocol requires every point to have at
/// least one field, but tags and the timestamp are optional.
///
/// # Examples
///
/// Creates a point with one tag and two fields:
///
/// ```
/// use telegraf::point;
///
/// let p = point!("measure", ("t1", "t1v"), ("f1", "f1v") ("f2", "f2v"));
/// ```
///
/// Creates a point with no tags and one field:
///
/// ```
/// use telegraf::point;
///
/// let p = point!("measure", ("f1", "f1v"));
/// ```
///
/// Creates a point with no tags, one field and a custom timestamp:
///
/// ```
/// use telegraf::point;
///
/// let p = point!("measure", ("f1", "f1v"); 100);
/// ```
#[macro_export]
macro_rules! point {
    ($measure:expr, $(($fname:expr, $fval:expr)) + $(; $ts:expr)?) => {
        {
            let mut fields: Vec<(String, Box<dyn $crate::IntoFieldData>)> = Vec::new();
            $(
                fields.push((String::from($fname), Box::new($fval)));
            )*

            #[allow(unused_mut)]
            let mut timestamp: Option<u64> = None;
            $(
                timestamp = timestamp.or(Some($ts));
            )?

            $crate::Point::new(
                String::from($measure),
                Vec::new(),
                fields,
                timestamp,
            )
        }
    };
    ($measure:expr, $(($tname:expr, $tval:expr)) +, $(($fname:expr, $fval:expr)) + $(; $ts:expr)?) => {
        {
            let mut tags: Vec<(String, String)> = Vec::new();
            let mut fields: Vec<(String, Box<dyn $crate::IntoFieldData>)> = Vec::new();
            $(
                tags.push((String::from($tname), format!("{}", $tval)));
            )*

            $(
                fields.push((String::from($fname), Box::new($fval)));
            )*

            #[allow(unused_mut)]
            let mut timestamp: Option<u64> = None;
            $(
                timestamp = timestamp.or(Some($ts));
            )?

            $crate::Point::new(
                String::from($measure),
                tags,
                fields,
                timestamp,
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn can_create_with_ts() {
        let p = point!("test", ("t", "v")("t2", "v2"), ("f", "v"); 10);
        let exp = Point::new(
            "test".to_string(),
            vec![
                ("t".to_string(), "v".to_string()),
                ("t2".to_string(), "v2".to_string()),
            ],
            vec![("f".to_string(), Box::new("v"))],
            Some(10),
        );
        assert_eq!(p, exp);
    }

    #[test]
    fn can_create_with_tag() {
        let p = point!("test", ("t", "v")("t2", "v2"), ("f", "v"));
        let exp = Point::new(
            "test".to_string(),
            vec![
                ("t".to_string(), "v".to_string()),
                ("t2".to_string(), "v2".to_string()),
            ],
            vec![("f".to_string(), Box::new("v"))],
            None,
        );
        assert_eq!(p, exp);
    }

    #[test]
    fn can_create_whtout_tag() {
        let p = point!("test", ("f", "v"));
        let exp = Point::new(
            "test".to_string(),
            Vec::new(),
            vec![("f".to_string(), Box::new("v"))],
            None,
        );
        assert_eq!(p, exp);
    }
}
