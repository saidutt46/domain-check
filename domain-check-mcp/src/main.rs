mod tools;

use rmcp::transport::stdio;
use rmcp::ServiceExt;
use tools::DomainCheckServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise tracing to stderr — stdout is reserved for MCP JSON-RPC.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "domain_check_mcp=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting domain-check MCP server");

    let server = DomainCheckServer::new();
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Failed to start MCP service: {e}");
    })?;
    service.waiting().await?;

    Ok(())
}
