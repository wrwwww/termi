use serde::Serializer;

pub fn serialize_optional_f32_with_two_decimal_places<S>(
    value: &Option<f32>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => {
            let rounded = (v * 100.0).round() / 100.0;
            let formatted = format!("{:.2}", rounded);
            let clean_value: f64 = formatted.parse().unwrap_or(rounded as f64);
            serializer.serialize_some(&clean_value)
        }
        None => serializer.serialize_none(),
    }
}
pub fn serialize_f32_with_two_decimal_places<S>(
    value: &f32,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded = (value * 100.0).round() / 100.0;
    let formatted = format!("{:.2}", rounded);
    let clean_value: f64 = formatted.parse().unwrap_or(rounded as f64);
    serializer.serialize_f64(clean_value)
}
