#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestFieldType {
    /// e.g. `req.body.x` => BodyField(["x"])
    BodyField,
    /// e.g. `req.params.user_id`
    ParamField,
    /// e.g. `req.query.user_id`
    QueryField,
    /// e.g. `req.headers.auth` / `req.headers` if None
    HeadersField,
}

impl core::fmt::Display for RequestFieldType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RequestFieldType::BodyField => write!(f, "req.body"),
            RequestFieldType::ParamField => write!(f, "req.params"),
            RequestFieldType::QueryField => write!(f, "req.query"),
            RequestFieldType::HeadersField => write!(f, "req.headers"),
        }
    }
}
