use color_eyre::eyre::Result;
use rmcp::{
    ErrorData as McpError, ServiceExt, handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::env;
use std::fs::OpenOptions;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum StatusResponse {
    Error {
        error: String,
    },
    Success {
        version: String,
        serial: u16,
        now: String,
        #[serde(rename = "lastUpdateStart")]
        last_update_start: String,
        #[serde(rename = "lastUpdateDone")]
        last_update_done: String,
        #[serde(rename = "lastUpdateDuration")]
        last_update_duration: f32,
    },
}

#[derive(Serialize, Deserialize)]
struct Vrp {
    pub asn: String,
    pub prefix: String,
    pub max_length: String,
}

#[derive(Serialize, Deserialize)]
struct VRPs {
    pub matched: Vec<Vrp>,
    pub unmatched_as: Vec<Vrp>,
    pub unmatched_length: Vec<Vrp>,
}

#[derive(Serialize, Deserialize)]
struct Validity {
    pub state: String,
    pub description: String,
    #[serde(rename = "VRPs")]
    pub vrps: VRPs,
}

#[derive(Serialize, Deserialize)]
struct Route {
    pub origin_asn: String,
    pub prefix: String,
}

#[derive(Serialize, Deserialize)]
struct ValidatedRoute {
    pub route: Route,
    pub validity: Validity,
}

#[derive(Serialize, Deserialize)]
struct ValidityResponse {
    pub validated_route: ValidatedRoute,
    #[serde(rename = "generatedTime")]
    pub generated_time: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ValidityArgs {
    #[schemars(description = "The Autonomous System Number (ASN) to validate")]
    asn: String,
    #[schemars(description = "The IP address prefix to validate (e.g., 192.0.2.0/24)")]
    prefix: String,
}

struct RPKITool {
    endpoint: String,
    tool_router: ToolRouter<RPKITool>,
}

#[tool_router]
impl RPKITool {
    fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Status of the RPKI relying party")]
    async fn status(&self) -> Result<CallToolResult, McpError> {
        match reqwest::get(format!("{}/api/v1/status", self.endpoint)).await {
            Ok(res) => match res.json::<StatusResponse>().await {
                Ok(status_data) => match serde_json::to_value(status_data) {
                    Ok(json_value) => Ok(CallToolResult::structured(json_value)),
                    Err(err) => Err(McpError {
                        code: ErrorCode(-1),
                        message: Cow::from(err.to_string()),
                        data: None,
                    }),
                },
                Err(err) => {
                    tracing::error!("{:?}", &err);
                    Err(McpError {
                        code: err
                            .status()
                            .map(|s| ErrorCode(s.as_u16() as i32))
                            .unwrap_or(ErrorCode(-1)),
                        message: Cow::from(err.to_string()),
                        data: None,
                    })
                }
            },
            Err(err) => {
                tracing::error!("{:?}", &err);
                Err(McpError {
                    code: err
                        .status()
                        .map(|s| ErrorCode(s.as_u16() as i32))
                        .unwrap_or(ErrorCode(-1)),
                    message: Cow::from(err.to_string()),
                    data: None,
                })
            }
        }
    }

    #[tool(
        description = "Returns a JSON object indicating whether a route announcement identified by its origin Autonomous System Number (ASN) and IP address prefix is RPKI valid, invalid, or not found. The response also includes the complete set of Validated ROA Payloads (VRPs) that determined this outcome"
    )]
    async fn validity(&self, args: Parameters<ValidityArgs>) -> Result<CallToolResult, McpError> {
        match reqwest::get(format!(
            "{}/api/v1/validity/{}/{}",
            self.endpoint, args.0.asn, args.0.prefix
        ))
        .await
        {
            Ok(res) => match res.json::<ValidityResponse>().await {
                Ok(status_data) => match serde_json::to_value(status_data) {
                    Ok(json_value) => Ok(CallToolResult::structured(json_value)),
                    Err(err) => Err(McpError {
                        code: ErrorCode(-1),
                        message: Cow::from(err.to_string()),
                        data: None,
                    }),
                },
                Err(err) => {
                    tracing::error!("{:?}", &err);
                    Err(McpError {
                        code: err
                            .status()
                            .map(|s| ErrorCode(s.as_u16() as i32))
                            .unwrap_or(ErrorCode(-1)),
                        message: Cow::from(err.to_string()),
                        data: None,
                    })
                }
            },
            Err(err) => {
                tracing::error!("{:?}", &err);
                Err(McpError {
                    code: err
                        .status()
                        .map(|s| ErrorCode(s.as_u16() as i32))
                        .unwrap_or(ErrorCode(-1)),
                    message: Cow::from(err.to_string()),
                    data: None,
                })
            }
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
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;

    // Initialize tracing subscriber to write to a file
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/rpki_mcp.log")?;

    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args: Vec<String> = env::args().collect();
    let endpoint = args[1].clone();
    let service = RPKITool::new(endpoint)
        .serve(stdio())
        .await
        .inspect_err(|e| {
            println!("Error starting server: {e}");
        })?;
    service.waiting().await?;

    Ok(())
}
