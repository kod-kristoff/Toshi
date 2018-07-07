use super::*;

use futures::{future, Future, Stream};

use std::collections::HashMap;
use std::io::Result as IOResult;
use std::panic::RefUnwindSafe;

#[derive(Deserialize, StateData, StaticResponseExtender, Debug)]
pub struct Search {
    pub query: Queries,

    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 5 }

#[derive(Deserialize, Debug, Clone)]
pub struct TermQuery {
    pub term: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TermsQuery {
    pub terms: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RangeQuery {
    pub range: HashMap<String, HashMap<String, i64>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RawQuery {
    pub raw: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Queries {
    TermQuery(TermQuery),
    TermsQuery(TermsQuery),
    RangeQuery(RangeQuery),
    RawQuery(RawQuery),
}

#[derive(Clone, Debug)]
pub struct SearchHandler {
    catalog: Arc<IndexCatalog>,
}

impl RefUnwindSafe for SearchHandler {}

impl SearchHandler {
    pub fn new(catalog: Arc<IndexCatalog>) -> Self { SearchHandler { catalog } }
}

impl Handler for SearchHandler {
    fn handle(self, mut state: State) -> Box<HandlerFuture> {
        let index = IndexPath::take_from(&mut state);
        let f = Body::take_from(&mut state).concat2().then(move |body| match body {
            Ok(b) => {
                let search: Search = serde_json::from_slice(&b).unwrap();
                info!("Query: {:#?}", search);
                let docs = match self.catalog.search_index(&index.index, &search) {
                    Ok(v) => v,
                    Err(e) => return handle_error(state, e),
                };
                info!("Query returned {} doc(s) on Index: {}", docs.len(), index.index);

                let data = Some((serde_json::to_vec_pretty(&docs).unwrap(), mime::APPLICATION_JSON));
                let resp = create_response(&state, StatusCode::Ok, data);
                future::ok((state, resp))
            }
            Err(e) => handle_error(state, e),
        });
        Box::new(f)
    }
}

new_handler!(SearchHandler);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializing() {
        let term_query = r#"{ "query" : { "term" : { "user" : "Kimchy" } } }"#;
        let terms_query = r#"{ "query": { "terms" : { "user" : ["kimchy", "elasticsearch"] } } }"#;
        let range_query = r#"{ "query": { "range" : { "age" : { "gte" : 10, "lte" : 20 } } } }"#;
        let raw_query = r#"{ "query" : { "raw" : "year:[1 TO 10]" } }"#;

        let parsed_term: Search = serde_json::from_str(term_query).unwrap();
        let parsed_terms: Search = serde_json::from_str(terms_query).unwrap();
        let parsed_range: Search = serde_json::from_str(range_query).unwrap();
        let parsed_raw: Search = serde_json::from_str(raw_query).unwrap();

        println!("{:#?}", parsed_term);
        println!("{:#?}", parsed_terms);
        println!("{:#?}", parsed_range);
        println!("{:#?}", parsed_raw);
    }
}
