use crate::{
    prelude::{AsyncContext, Plugin, ThunkContext, Value},
    state::AttributeIndex,
};
use hyper::{Body, Request as HttpRequest};
use reality::{BlockObject, BlockProperties};
use tracing::{event, Level};

use super::AddDoc;

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
        if let Some(mut docs) = Self::start_docs(parser) {
            let docs = &mut docs;
            /*
            Example Usage:
                : Accept .header text/json
                or
                : Accept .symbol {media_type}
                : .fmt media_type
             */
            docs.as_mut()
                .add_custom_with("header", |p, c| {
                    let var_name = p.symbol().expect("Requires a var name").to_string();

                    let last = p
                        .last_child_entity()
                        .expect("should have added an entity for the process");

                    p.define_child(last, "header", Value::Symbol(var_name.to_string()));
                    p.define_child(last, var_name, Value::Symbol(c));
                })
                .add_doc(docs, "Sets an http header on the request.")
                .name_required()
                .list()
                .symbol("Should be a valid header value");

            docs.as_mut()
                .add_custom_with("accept", |p, c| {
                    let last = p
                        .last_child_entity()
                        .expect("should have added an entity for the process");

                    p.define_child(last, "header", "accept");
                    p.define_child(last, "accept", Value::Symbol(c));
                })
                .add_doc(docs, "Shortcut for `: Accept .header {value}`")
                .symbol("Should be the value of the Accept header, which is usually a media type");

            docs.as_mut()
                .add_custom_with("get", |p, _| {
                    let last = p
                        .last_child_entity()
                        .expect("should have added an entity for the process");

                    p.define_child(last, "method", "GET");
                })
                .add_doc(docs, "Sets the method for the request to GET (defaults)");

            docs.as_mut()
                .add_custom_with("post", |p, _| {
                    let last = p
                        .last_child_entity()
                        .expect("should have added an entity for the process");

                    p.define_child(last, "method", "POST");
                })
                .add_doc(docs, "Sets the method for the request to POST");

            docs.as_mut()
                .add_custom_with("put", |p, _| {
                    let last = p
                        .last_child_entity()
                        .expect("should have added an entity for the process");

                    p.define_child(last, "method", "PUT");
                })
                .add_doc(docs, "Sets the method for the request to PUT");

            docs.as_mut()
                .add_custom_with("head", |p, _| {
                    let last = p
                        .last_child_entity()
                        .expect("should have added an entity for the process");

                    p.define_child(last, "method", "HEAD");
                })
                .add_doc(docs, "Sets the method for the request to HEAD");

            docs.as_mut()
                .add_custom_with("delete", |p, _| {
                    let last = p
                        .last_child_entity()
                        .expect("should have added an entity for the process");

                    p.define_child(last, "method", "DELETE");
                })
                .add_doc(docs, "Sets the method for the request to DELETE");
        }
    }

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
        context.clone().task(|_| {
            let mut tc = context.clone();
            async move {
                if let Some(client) = tc.client() {
                    let mut request = HttpRequest::builder();

                    if let Some(uri) = tc.state().find_symbol("request") {
                        if !uri.is_empty() {
                            request = request.uri(uri);
                        } else if let Some(uri) = tc.search().find_symbol("uri") {
                            request = request.uri(uri);
                        } else if let Some(uri) = tc.search().find_symbol("api") {
                            request = request.uri(uri);
                        }
                    }

                    if let Some(method) = tc.search().find_symbol("method") {
                        request = request.method(method.as_str());
                    }

                    for name in tc.search().find_symbol_values("header") {
                        if let Some(header_value) = tc.search().find_symbol(&name) {
                            let header_value = tc.format(header_value);

                            request = request.header(name, header_value);
                        }
                    }

                    // Allow previous plugins to configure headers
                    if let Some(previous) = tc.previous() {
                        for name in previous.find_symbol_values("header") {
                            if let Some(header_value) = tc.search().find_symbol(&name) {
                                let header_value = tc.format(header_value);

                                if let Some(headers) = request.headers_ref() {
                                    if !headers.contains_key(&name) {
                                        request = request.header(name, header_value);
                                    }
                                }
                            }
                        }
                    }

                    let body = tc
                        .search()
                        .find_binary("body")
                        .and_then(|b| Some(Body::from(b)))
                        .unwrap_or(Body::empty());

                    event!(Level::TRACE, "{:#?}", request);

                    match request.body(body) {
                        Ok(request) => {
                            let uri = request.uri().clone();
                            match client.request(request).await {
                                Ok(resp) => {
                                    tc.cache_response(resp);
                                }
                                Err(err) => {
                                    event!(
                                        Level::ERROR,
                                        "request: error sending request {err}, {:?}",
                                        uri
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            event!(Level::ERROR, "request: error creating request {err}");
                        }
                    }
                }

                tc.copy_previous();
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
