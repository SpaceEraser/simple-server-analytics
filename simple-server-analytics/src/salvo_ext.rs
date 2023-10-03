pub mod handler;
pub mod listener;
pub mod service;

use std::sync::Arc;

use salvo::Router;

use super::SimpleAnalytics;

impl SimpleAnalytics {
    pub fn append_routes(&self, router: &mut salvo::Router) {
        router.routers_mut().push(Router::with_path("/analytics"))
    }

    pub fn prepend_handler(&self, router: &mut salvo::Router) {
        router
            .hoops_mut()
            .insert(0, Arc::new(handler::SimpleAnalyticsHandler::new(self)));
    }
}
