//! MCP server for zefix!: lets AI assistants simulate Schafkopf scenarios over stdio.

mod server;
mod tools;

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = server::Zefix.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
