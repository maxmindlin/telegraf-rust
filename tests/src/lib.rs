use telegraf_derive::*;
use telegraf::*;

#[derive(Metric)]
struct NoTags {
    i: i32,
}

#[derive(Metric)]
struct Tags {
    i: i32,
    #[telegraf(tag)]
    t: String,
    f: f32,
}

#[derive(Metric)]
struct TagsWithLifetime<'a> {
    i: f32,
    #[telegraf(tag)]
    t: &'a str,
}

#[derive(Metric)]
#[measurement = "custom"]
struct CustomMeasurementName {
    i: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_derive_without_tags() {
        let s = NoTags { i: 1 };
        let exp = point!("NoTags", ("i", 1));
        assert_eq!(s.to_point(), exp);
    }

    #[test]
    fn can_derive_with_tags() {
        let s = Tags { i: 1, t: "t".to_string(), f: 2. };
        let exp = point!("Tags", ("t", "t"), ("i", 1) ("f", 2.));
        assert_eq!(s.to_point(), exp);
    }

    #[test]
    fn can_derive_with_lifetimes() {
        let s = TagsWithLifetime { i: 1., t: "t" };
        let exp = point!("TagsWithLifetime", ("t", "t"), ("i", 1.));
        assert_eq!(s.to_point(), exp);
    }

    #[test]
    fn can_derive_with_meaurement_attr() {
        let s = CustomMeasurementName { i: 1 };
        let exp = point!("custom", ("i", 1));
        assert_eq!(s.to_point(), exp);
    }
}
