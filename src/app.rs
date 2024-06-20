use async_trait::async_trait;
use lambda_http::Request;
use std::collections::HashMap;

pub struct Application<T>
where
	T: Send + Sync,
{
	handlers: Vec<Handler<T>>,
}

// Define the macro
#[macro_export]
macro_rules! page {
    ($page:ident) => {
        Box::new($page) as Box<dyn Page<_> + Send + Sync>
    };
}
#[macro_export]

macro_rules! route {
    ($path:expr) => {
        |req| req.uri().path() == $path
    };
}

pub struct Cors;

#[async_trait]
impl<T:Send+Sync> Page<T> for Cors {
	async fn render(&self, _req: &Request, _resources: &mut T) -> AppResponse {
		AppResponse {
			body: Vec::from([]),
			status_code: 200,
			headers: vec![
				("Access-Control-Allow-Origin".to_string(), "*".to_string()),
				(
					"Access-Control-Allow-Methods".to_string(),
					"GET, POST, OPTIONS".to_string(),
				),
				(
					"Access-Control-Allow-Headers".to_string(),
					"Content-Type".to_string(),
				),
			],
			cookies: Default::default(),
		}
	}
}

//for eg Handler::new(route!("/signup"), page!(Signup))
#[macro_export]
macro_rules! handler {
    ($path:expr, $page:ident) => {
        Handler::new(route!($path), page!($page))
    };
    //i hate this with a passion, but if i dont have the expr version
    //rustrover wil refuse to understand the macro properly because im using a unit struct
    //which can be interpreted as a ident or an expr i guess?
    //having both allows rustrover to detect one, but cargo/rustc build based on the other
    ($path:expr, $page:expr) => {
        Handler::new(route!($path), page!($page))
    };
}

#[macro_export]
macro_rules! cors {
    () => {
        Handler::new(
            |req| req.method() == Method::OPTIONS,
            Box::new(Cors) as Box<dyn Page<_> + Send + Sync>,
        )
    };
}
// on 200 we load the 200.html file, which is statically included to avoid fs overhead. it is always relative to the location we include it, so we can use the static  file include
#[macro_export]
macro_rules! is200 {
    () => {
        include_str!("./200.html")
    };
}
#[macro_export]
macro_rules! is4xx {
    ($n:expr) => {
        include_str!(concat!("./", $n, ".html"))
    };
}
use std::fmt;

#[derive(Debug)]
pub struct Cookie {
	pub name: String,
	pub value: String,
	pub secure: bool,
	pub httponly: bool,
}

#[derive(Default)]
pub struct CookieJar {
	cookies: Vec<Cookie>,
}

impl CookieJar {
	pub fn add_cookie(&mut self, name: &str, value: &str, secure: bool, httponly: bool) {
		let cookie = Cookie {
			name: name.to_string(),
			value: value.to_string(),
			secure,
			httponly,
		};
		self.cookies.push(cookie);
	}

	pub fn eat_cookies(&self) -> (String, String) {
		let mut header = String::new();

		for cookie in &self.cookies {
			if !header.is_empty() {
				header.push_str("; ");
			}

			header.push_str(&format!("{}={}", cookie.name, cookie.value));

			if cookie.secure {
				header.push_str("; Secure");
			}

			if cookie.httponly {
				header.push_str("; HttpOnly");
			}
		}

		// Returning a tuple of (String, String)
		("Set-Cookie".to_string(), header)
	}
}

impl fmt::Display for CookieJar {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let (header, value) = self.eat_cookies();
		write!(f, "{}={}", header, value)
	}
}

pub struct AppResponse {
	pub status_code: u16,
	pub headers: Vec<(String, String)>,
	pub body: Vec<u8>,
	pub cookies: CookieJar,
}

impl AppResponse {
	pub fn new(status_code: u16, headers: Vec<(String, String)>, body: Vec<u8>) -> Self {
		AppResponse {
			status_code,
			headers,
			body,
			cookies: Default::default(),
		}
	}

	pub fn eat_cookies(&mut self) {
		self.headers.push(self.cookies.eat_cookies());

		dbg!(&self.headers);
	}
}

type InnerData = Option<HashMap<String, String>>;

pub struct Handler<T>
where
	T: Send + Sync,
{
	pub page: Box<dyn Page<T> + Send + Sync>,
	inner_data: InnerData,
	data_match: Option<fn(&Request, &InnerData) -> bool>,
	simple_match: Option<fn(&Request) -> bool>,
}

#[async_trait]
pub trait Page<T>
where
	T: Send + Sync,
{
	async fn render(&self, req: &Request, resources: &mut T) -> AppResponse;
}

impl<T> Handler<T>
where
	T: Send + Sync,
{
	pub fn match_request(&self, request: &Request) -> bool {
		let funcs = (&self.data_match, &self.simple_match);
		match funcs {
			(Some(data), None) => data(request, &self.inner_data),
			(None, Some(simple)) => simple(request),
			_ => false,
		}
	}
	pub fn new(match_request: fn(&Request) -> bool, page: Box<dyn Page<T> + Send + Sync>) -> Self {
		Handler {
			data_match: None,
			page,
			inner_data: None,
			simple_match: Some(match_request),
		}
	}
	fn new_inner(
		match_request: fn(&Request, &InnerData) -> bool,
		page: Box<dyn Page<T> + Send + Sync>,
		inner_data: Option<HashMap<String, String>>,
	) -> Self {
		Handler {
			data_match: Some(match_request),
			simple_match: None,
			// match_request,
			page,
			inner_data,
		}
	}

	pub fn get(path: &str, page: Box<dyn Page<T> + Send + Sync>) -> Self {
		let mut d = HashMap::<String, String>::new();
		d.insert("path".to_string(), path.to_string());

		Handler::new_inner(
			|request, d| {
				let path = d.as_ref().unwrap().get("path").unwrap();

				request.method() == "GET" && request.uri().path() == path
			},
			page,
			Some(d),
		)
	}
	pub fn post(path: &str, page: Box<dyn Page<T> + Send + Sync>) -> Self {
		let mut d = HashMap::<String, String>::new();
		d.insert("path".to_string(), path.to_string());

		Handler::new_inner(
			|request, d| {
				let path = d.as_ref().unwrap().get("path").unwrap();
				request.method() == "POST" && request.uri().path() == path
			},
			page,
			Some(d),
		)
	}
	pub fn put(path: &str, page: Box<dyn Page<T> + Send + Sync>) -> Self {
		let mut d = HashMap::<String, String>::new();
		d.insert("path".to_string(), path.to_string());

		Handler::new_inner(
			|request, d| {
				let path = d.as_ref().unwrap().get("path").unwrap();
				request.method() == "PUT" && request.uri().path() == path
			},
			page,
			Some(d),
		)
	}
}

impl<T> Default for Application<T>
where
	T: Send + Sync,
{
	fn default() -> Self {
		Application::new()
	}
}

impl<T> Application<T>
where
	T: Send + Sync,
{
	pub fn new() -> Self {
		Application {
			handlers: Vec::new(),
		}
	}

	pub fn from(handlers: Vec<Handler<T>>) -> Self {
		Application { handlers }
	}

	pub fn register(&mut self, handler: Handler<T>) {
		self.handlers.push(handler);
	}

	pub async fn handle_request(&self, request: &Request, resources: &mut T) -> AppResponse {
		dbg!(&request);
		for handler in &self.handlers {
			if handler.match_request(request) {
				return handler.page.render(request, resources).await;
			}
		}

		AppResponse {
			status_code: 404,
			headers: Vec::new(),
			body: Vec::new(),
			cookies: Default::default(),
		}
	}
}
