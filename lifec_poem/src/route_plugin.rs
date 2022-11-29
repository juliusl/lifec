use lifec::prelude::ThunkContext;
use poem::{Response, RouteMethod};

/// Plugin w/ a route, that handles the output of a plugin call sequence,
///
pub trait RoutePlugin {
    /// Returns a route for this plugin,
    ///
    /// Generally, plugins are stateless, but this trait will likely be used in conjunction with the WebApp trait. This means that, there will
    /// be a start-up phase of the app host that gives implementations, the chance to initialize/customize a route.
    ///
    fn route(&self, route_method: Option<RouteMethod>) -> RouteMethod;

    /// Returns a response from the context,
    ///
    /// Override to customize the response
    ///
    fn response(context: &mut ThunkContext) -> Response {
        context
            .take_response()
            .expect("should have a response")
            .into()
    }
}
