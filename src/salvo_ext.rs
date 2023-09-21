pub mod listener;
pub mod service;

use salvo::Router;

use super::SimpleAnalytics;

impl SimpleAnalytics {
    pub fn append_routes(router: &mut salvo::Router) {
        router.routers_mut().push(Router::with_path("/analytics"))
    }
}
