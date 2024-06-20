pub mod app;

pub mod prelude {
    pub use http::method::Method;
    use lambda_http::http;
    pub use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};
    pub use lambda_runtime::{run as run_t, service_fn as service_fn_t, LambdaEvent};
    pub use lazy_static::lazy_static;
    pub use log::Level;
    pub use std::sync::Arc;
    pub use std::sync::Mutex;
    pub use tracing as trace;
    pub use tracing::*;
    pub use tracing_subscriber::fmt;
    pub use tracing_subscriber::prelude::*;
}

//many things have been renamed to match the prelude pub uses so that the consumer of the macro doesn't have to import them
//or add the extra dependencies to every crate

#[macro_export]
macro_rules! _lambda_main {
        ($handler:ident) => {
            #[tokio::main]
            async fn main() -> Result<(), Error> {
                tracing_subscriber::fmt()
                    .with_max_level(tracing::Level::INFO)
                    .with_target(false)
                    .without_time()
                    .init();

                run(service_fn($handler)).await
            }
        };
    }

// Define a custom procedural macro for lambda
#[macro_export]
macro_rules! _handler {
    ($($name:ident : $event_type:ty),* $(,) ? => { $($body:tt)* }) => {
        async fn function_handler($($name : $event_type),*) -> Result<Response<Body>, Error> {
            $($body)*
        }
    };
}
#[macro_export]
macro_rules! await_resource {
    ($resource_func:ident) => {
        //aaaaa
        $resource_func()
    };
    ($resource_func:ident async) => {
        //bbbbbb $resource_func async
        $resource_func().await
    };
}
// #[macro_export]
#[macro_export]
macro_rules! lambda_http_handler {
    // With shared resources
    ($log_level:ident, $event:ident : $event_type:ty, $($resources:ident : $resource_type:ty = $($resource_func:expr)*),*, () => { $($body:tt)* }) => {

        async fn function_handler($event: $event_type, $($resources: &$resource_type),*) -> Result<Response<Body>, Error> {
            $($body)*
        }

        #[tokio::main]
        async fn main() -> Result<(), Error> {
            fmt()
                .with_max_level(trace::Level::$log_level)
                .with_target(false)
                .without_time()
                .init();

             $(
                let $resources = $($resource_func)*;
            )*

            run(service_fn(|e| function_handler(e, $(&$resources),*))).await?;
            Ok(())
        }
    };


    // Without shared resources
    ($log_level:ident, $event:ident : $event_type:ty => { $($body:tt)* }) => {
        async fn function_handler($event: $event_type) -> Result<Response<Body>, Error> {
            $($body)*
        }

        #[tokio::main]
        async fn main() -> Result<(), Error> {
            fmt()
                .with_max_level(trace::Level::$log_level)
                .with_target(false)
                .without_time()
                .init();

            run(service_fn(function_handler)).await?;
            Ok(())
        }
    };
}
// macro_rules! lambda_http_handler {


#[macro_export]
macro_rules! lambda_handler {
    ($log_level:ident, <$event_type:ty, $r_type:ty>( $event:ident ) => { $($body:tt)* }) => {
        async fn function_handler($event: LambdaEvent<$event_type>) -> Result<$r_type, Error> {
            $($body)*
        }

        #[tokio::main]
        async fn main() -> Result<(), Error> {
            fmt()
                .with_max_level(trace::Level::$log_level)
                .with_target(false)
                .without_time()
                .init();

            //_t variants are for <T> that the lambda event uses. the non _t variants are for the lambda_http module/events which don't use <T>
            //but export equivalent functions with same name but different types

            run_t(service_fn_t(function_handler)).await
        }
    };
}
