use std::future::{ready, Ready};

use actix_web::dev::{forward_ready, Service, Transform};

pub struct JwtAuth;

impl<S: Service<Req>, Req> Transform<S, Req> for JwtAuth {
    type Response = S::Response;
    type Error = S::Error;
    type Transform = JwtAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware { service }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: S,
}

impl<S: Service<Req>, Req> Service<Req> for JwtAuthMiddleware<S> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    forward_ready!(service);

    fn call(&self, req: Req) -> Self::Future {
        self.service.call(req)
    }
}
