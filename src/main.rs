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

#[derive(Serialize, Deserialize)]
struct Roa {
    pub asn: String,
    pub prefix: String,
    #[serde(rename = "maxLength")]
    pub max_length: i64,
    pub ta: String,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    pub generated: i64,
    #[serde(rename = "generatedTime")]
    pub generated_time: String,
}

#[derive(Serialize, Deserialize)]
struct Roas {
    pub metadata: Metadata,
    pub roas: Vec<Roa>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RoasArgs {
    #[schemars(description = "The Autonomous System Number (ASN) to retrieve ROAs for")]
    asn: String,
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

    /// Generic helper to fetch JSON from an endpoint and return it as a CallToolResult
    async fn fetch_json_response<T>(url: String) -> Result<CallToolResult, McpError>
    where
        T: for<'de> Deserialize<'de> + Serialize,
    {
        let res = reqwest::get(&url).await.map_err(|err| {
            tracing::error!("Request failed: {:?}", err);
            McpError {
                code: err
                    .status()
                    .map(|s| ErrorCode(s.as_u16() as i32))
                    .unwrap_or(ErrorCode(-1)),
                message: Cow::from(format!("Request failed: {err}")),
                data: None,
            }
        })?;

                if !res.status().is_success() {
                    let status_code = res.status().as_u16() as i32;
                    let error_text = res
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::error!("HTTP error {}: {}", status_code, error_text);
                    return Err(McpError {
                        code: ErrorCode(status_code),
                        message: Cow::from(error_text),
                        data: None,
                    });
                }

        let data = res.json::<T>().await.map_err(|err| {
                        tracing::error!("Failed to parse JSON response: {:?}", err);
            McpError {
                            code: err
                                .status()
                                .map(|s| ErrorCode(s.as_u16() as i32))
                                .unwrap_or(ErrorCode(-1)),
                            message: Cow::from(format!("Failed to parse response: {err}")),
                            data: None,
                    }
        })?;

        let json_value = serde_json::to_value(data).map_err(|err| {
            tracing::error!("Failed to serialize response: {:?}", err);
            McpError {
                code: ErrorCode(-1),
                message: Cow::from(format!("Failed to serialize response: {err}")),
                    data: None,
            }
        })?;

        Ok(CallToolResult::structured(json_value))
    }

    #[tool(description = "Status of the RPKI relying party")]
    async fn status(&self) -> Result<CallToolResult, McpError> {
        Self::fetch_json_response::<StatusResponse>(format!("{}/api/v1/status", self.endpoint))
            .await
    }

    #[tool(
        description = "Returns a JSON object indicating whether a route announcement identified by its origin Autonomous System Number (ASN) and IP address prefix is RPKI valid, invalid, or not found. The response also includes the complete set of Validated ROA Payloads (VRPs) that determined this outcome"
    )]
    async fn validity(&self, args: Parameters<ValidityArgs>) -> Result<CallToolResult, McpError> {
        Self::fetch_json_response::<ValidityResponse>(format!(
            "{}/api/v1/validity/{}/{}",
            self.endpoint, args.0.asn, args.0.prefix
        ))
        .await
    }

    #[tool(
        description = "Retrieves all Route Origin Authorizations (ROAs) for a given Autonomous System Number (ASN). Returns a JSON object containing metadata and a list of ROAs associated with the specified ASN"
    )]
    async fn roas(&self, args: Parameters<RoasArgs>) -> Result<CallToolResult, McpError> {
        Self::fetch_json_response::<Roas>(format!(
            "{}/json?select-asn={}",
            self.endpoint, args.0.asn
        ))
        .await
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
