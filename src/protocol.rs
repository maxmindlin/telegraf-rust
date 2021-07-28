#[derive(Debug)]
pub struct LineProtocol(String);

impl LineProtocol {
    pub fn new(
        measurement: String,
        tags: String,
        fields: String,
    ) -> Self {
        Self(format!("{},{} {}\n", measurement, tags, fields))
    }

    pub fn to_str(&self) -> &str {
        &self.0
    }
}

/// Used to convert Rust types to influx types. Must be
/// implemented by any type that will be used as a Field in a [Point].
pub trait IntoFieldData {
    fn into_field_data(&self) -> FieldData;
}

impl IntoFieldData for i32 {
    fn into_field_data(&self) -> FieldData {
        FieldData::Number(*self as i64)
    }
}

impl IntoFieldData for i64 {
    fn into_field_data(&self) -> FieldData {
        FieldData::Number(*self)
    }
}

impl IntoFieldData for f32 {
    fn into_field_data(&self) -> FieldData {
        FieldData::Float(*self as f64)
    }
}

impl IntoFieldData for f64 {
    fn into_field_data(&self) -> FieldData {
        FieldData::Float(*self)
    }
}

impl IntoFieldData for &str {
    fn into_field_data(&self) -> FieldData {
        FieldData::Str(String::from(*self))
    }
}

impl IntoFieldData for String {
    fn into_field_data(&self) -> FieldData {
        FieldData::Str(self.to_string())
    }
}


/// Influx types that can be used in a field.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldData {
    Number(i64),
    Float(f64),
    Str(String),
}

pub fn get_field_string(value: &FieldData) -> String {
    match value {
        FieldData::Number(n) => format!("{}i", n),
        FieldData::Float(f)  => format!("{}", f),
        FieldData::Str(s)    => format!(r#""{}""#, s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub name:  String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name:  String,
    pub value: FieldData,
}

#[derive(Debug)]
pub enum Attr {
    Tag(Tag),
    Field(Field)
}

pub fn format_attr(attrs: Vec<Attr>) -> String {
    let mut out: Vec<String> = attrs.into_iter()
        .map(|a| match a {
            Attr::Tag(t) => format!("{}={}", t.name, t.value),
            Attr::Field(f) => format!("{}={}", f.name, get_field_string(&f.value)),
        })
        .collect();
    out.sort();
    out.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_string_field_string() {
        let s = get_field_string(&FieldData::Str(String::from("hello")));
        assert_eq!(s, String::from(r#""hello""#));
    }

    #[test]
    fn can_format_field_attr() {
        let v1: Vec<Attr> = vec![
            Attr::Field(Field { name: String::from("f1"), value: FieldData::Number(1) }),
            Attr::Field(Field { name: String::from("f2"), value: FieldData::Number(2) })
        ];

        let v2: Vec<Attr> = vec![
            Attr::Field(Field { name: String::from("f1"), value: FieldData::Number(1) }),
            Attr::Field(Field { name: String::from("f2"), value: FieldData::Str(String::from("2")) })
        ];
        assert_eq!(format_attr(v1), String::from("f1=1i,f2=2i"));
        assert_eq!(format_attr(v2), String::from("f1=1i,f2=\"2\""));
    }
}
