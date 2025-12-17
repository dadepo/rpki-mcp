use color_eyre::eyre::Result;
use rmcp::{
    ServiceExt, handler::server::router::tool::ToolRouter, model::*, tool_handler, tool_router,
    transport::stdio,
};

pub struct RPKITool {
    tool_router: ToolRouter<RPKITool>,
}

#[tool_router]
impl RPKITool {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler]
impl rmcp::ServerHandler for RPKITool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP server that exposes functionalities of RPKI relay parties".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "rpki-mcp".into(),
                version: "0.1.0".into(),
                title: Some("MCP server for RPKI".into()),
                ..Implementation::default()
            },
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let service = RPKITool::new().serve(stdio()).await.inspect_err(|e| {
        println!("Error starting server: {e}");
    })?;
    service.waiting().await?;

    Ok(())
}
