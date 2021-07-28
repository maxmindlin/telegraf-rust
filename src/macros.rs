/// Shorthand macro for generating
/// [Point] structs.
#[macro_export]
macro_rules! point {
    ($measure:expr, $(($fname:expr, $fval:expr)) +) => {
        {
            use $crate::IntoFieldData;
            let mut fields: Vec<(String, Box<dyn IntoFieldData>)> = Vec::new();
            $(
                fields.push((String::from($fname), Box::new($fval)));
            )*

            $crate::Point::new(
                String::from($measure),
                Vec::new(),
                fields,
            )
        }
    };
    ($measure:expr, $(($tname:expr, $tval:expr)) +, $(($fname:expr, $fval:expr)) +) => {
        {
            use $crate::{IntoFieldData, Point};
            let mut tags: Vec<(String, String)> = Vec::new();
            let mut fields: Vec<(String, Box<dyn IntoFieldData>)> = Vec::new();
            $(
                tags.push((String::from($tname), String::from($tval)));
            )*

            $(
                fields.push((String::from($fname), Box::new($fval)));
            )*

            Point::new(
                String::from($measure),
                tags,
                fields,
            )
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn can_create_with_tag() {
        let p = point!("test", ("t", "v") ("t2", "v2"), ("f", "v"));
        let exp = Point::new(
            "test".to_string(),
            vec![
                ("t".to_string(), "v".to_string()),
                ("t2".to_string(), "v2".to_string()),
            ],
            vec![
                ("f".to_string(), Box::new("v")),
            ],
        );
        assert_eq!(p, exp);
    }

    #[test]
    fn can_create_whtout_tag() {
        let p = point!("test", ("f", "v"));
        let exp = Point::new(
            "test".to_string(),
            Vec::new(),
            vec![
                ("f".to_string(), Box::new("v")),
            ],
        );
        assert_eq!(p, exp);
    }
}
