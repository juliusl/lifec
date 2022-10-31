use hyper::{Body, Request as HttpRequest};
use reality::{BlockObject, BlockProperties};
use crate::{
    prelude::{AsyncContext, Plugin, ThunkContext, Value},
    state::AttributeIndex,
};

/// Type for installing a lifec plugin implementation. This plugin makes
/// https requests, with a hyper secure client.
///
#[derive(Default)]
pub struct Request;

impl Plugin for Request {
    fn symbol() -> &'static str {
        "request"
    }

    fn description() -> &'static str {
        "Creates a http request, and sends a request with a hyper client. HTTPS only"
    }

    fn compile(parser: &mut crate::prelude::AttributeParser) {
        /*
        Example Usage: 
            : Accept .header text/json
            or
            : Accept .symbol {media_type}
            : .fmt media_type
         */
        parser.add_custom_with("header", |p, c| {
            let var_name = p.symbol().expect("Requires a var name").to_string();

            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "header", Value::Symbol(var_name.to_string()));
            p.define_child(last, var_name, Value::Symbol(c));
        });

        parser.add_custom_with("get", |p, _| {
            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "method", "GET");
        });

        parser.add_custom_with("post", |p, _| {
            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "method", "POST");
        });

        parser.add_custom_with("put", |p, _| {
            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "method", "PUT");
        });

        parser.add_custom_with("head", |p, _| {
            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "method", "HEAD");
        });

        parser.add_custom_with("delete", |p, _| {
            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "method", "DELETE");
        });
    }

    fn call(context: &ThunkContext) -> Option<AsyncContext> {
        context.clone().task(|_| {
            let mut tc = context.clone();
            async move {
                if let Some(client) = tc.client() {
                    let mut request = HttpRequest::builder();

                    if let Some(uri) = tc.state().find_symbol("request") {
                        request = request.uri(uri);
                    } else if let Some(uri) = tc.search().find_symbol("uri") {
                        request = request.uri(uri);
                    } else if let Some(uri) = tc.search().find_symbol("api") {
                        request = request.uri(uri);
                    }

                    if let Some(method) = tc.state().find_symbol("method") {
                        request = request.method(method.as_str());
                    }
              
                    for name in tc.search().find_symbol_values("header") {
                        if let Some(header_value) = tc.state().find_symbol(&name) {
                            let header_value = tc.format(header_value);

                            request = request.header(name, header_value);
                        }
                    }

                    let body = tc
                        .state()
                        .find_binary("body")
                        .and_then(|b| Some(Body::from(b)))
                        .unwrap_or(Body::empty());

                    match request.body(body) {
                        Ok(request) => match client.request(request).await {
                            Ok(resp) => {
                                tc.cache_response(resp);
                            },
                            Err(err) => {
                                eprintln!("request: error sending request {err}");
                            }
                        },
                        Err(err) => {
                            eprintln!("request: error creating request {err}");
                        }
                    }
                }

                Some(tc)
            }
        })
    }
}

impl BlockObject for Request {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .optional("request")
            .optional("uri")
            .optional("header")
            .optional("method")
            .optional("fmt")
            .optional("get")
            .optional("head")
            .optional("post")
            .optional("put")
            .optional("delete")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}