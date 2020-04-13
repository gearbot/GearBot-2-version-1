use twilight::cache::InMemoryCache;
use twilight::command_parser::Parser;
use twilight::gateway::Cluster;
use twilight::http::Client as HttpClient;

#[derive(Debug)]
pub struct Context<'a> {
    pub command_parser: Parser<'a>,
    pub cache: InMemoryCache,
    pub cluster: Cluster,
    pub http: HttpClient,
}

impl<'a> Context<'a> {
    pub fn new(
        parser: Parser<'a>,
        cache: InMemoryCache,
        cluster: Cluster,
        http: HttpClient,
    ) -> Self {
        Context {
            command_parser: parser,
            cache,
            cluster,
            http,
        }
    }
}