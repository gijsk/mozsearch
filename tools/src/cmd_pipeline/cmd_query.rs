use async_trait::async_trait;
use serde_json::{to_value};
use structopt::StructOpt;

use super::{interface::{JsonValue, PipelineCommand, PipelineValues}, builder::build_pipeline_graph};
use crate::{
    abstract_server::{AbstractServer, Result}, query::chew_query::chew_query,
};

/// Run a new-style `query-parser` `term:value` query parse against the local
/// index.  Remote server is currently a no-op, but when supported the entire
/// query will be run on the server (because we want to test the server).
#[derive(Debug, StructOpt)]
pub struct Query {
    /// Query string
    query: String,

    /// Output the constructed pipeline instead of running the pipeline.
    #[structopt(short, long)]
    dump_pipeline: bool,
}

#[derive(Debug)]
pub struct QueryCommand {
    pub args: Query,
}


#[async_trait]
impl PipelineCommand for QueryCommand {
    async fn execute(
        &self,
        server: &Box<dyn AbstractServer + Send + Sync>,
        _input: PipelineValues,
    ) -> Result<PipelineValues> {
        let pipeline_plan = chew_query(&self.args.query)?;

        if self.args.dump_pipeline {
            return Ok(PipelineValues::JsonValue(JsonValue { value: to_value(pipeline_plan)? }));
        }

        let graph = build_pipeline_graph(server.clonify(), pipeline_plan)?;

        let result = graph.run(true).await;
        result
    }
}
