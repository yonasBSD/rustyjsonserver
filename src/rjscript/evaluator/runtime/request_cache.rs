use crate::{http::request::Request, rjscript::{
    ast::position::Position,
    evaluator::{runtime::value::RJSValue, EvalResult},
}};

#[derive(Clone)]
pub struct RequestCache {
    pub body: RJSValue,
    pub route_params: RJSValue,
    pub query_params: RJSValue,
    pub headers: RJSValue,
}

impl RequestCache {
    pub fn from_request(req: Request) -> EvalResult<Self> {
        let pos = Position::default();
        Ok(Self {   
            body: RJSValue::json_to_rjs(&req.body, pos)?,
            route_params: RJSValue::string_map_to_rjs(&req.route_params),
            query_params: RJSValue::string_map_to_rjs(&req.query_params),
            headers: RJSValue::string_map_to_rjs(&req.headers),
        })
    }

    #[inline] pub fn body(&self) -> RJSValue { self.body.clone() }
    #[inline] pub fn route_params(&self) -> RJSValue { self.route_params.clone() }
    #[inline] pub fn query_params(&self) -> RJSValue { self.query_params.clone() }
    #[inline] pub fn headers(&self) -> RJSValue { self.headers.clone() }
}
