//! rmcp-Anbindung: [`FirepixelMcp`] setzt `ServerHandler` um und leitet Werkzeugaufrufe über den
//! [`SessionHost`] an die Sitzung weiter.

use crate::host::{SessionHost, with_session};
use crate::resources;
use crate::tools::{self, Block, ToolEnv};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ServerCapabilities,
    ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;
use std::sync::Arc;

/// Hinweise für Agenten in der `initialize`-Antwort (`msg-mcp-instructions*` in der aktuellen
/// Sprache; der Server ruft sie in der Sprache seiner Werkzeug-Umgebung auf).
pub fn instructions(bridge: bool) -> String {
    let extra = if bridge {
        "msg-mcp-instructions-bridge"
    } else {
        "msg-mcp-instructions-headless"
    };
    format!(
        "{} {}",
        fp_i18n::t("msg-mcp-instructions"),
        fp_i18n::t(extra)
    )
}

/// Der MCP-Server (eine Instanz je Verbindung; der Zustand liegt im Host).
#[derive(Clone)]
pub struct FirepixelMcp {
    host: Arc<dyn SessionHost>,
    env: ToolEnv,
}

impl FirepixelMcp {
    /// Neuer Server auf einem Sitzungs-Host.
    pub fn new(host: Arc<dyn SessionHost>) -> FirepixelMcp {
        FirepixelMcp {
            host,
            env: ToolEnv::default(),
        }
    }

    /// Mit eigener Werkzeug-Umgebung (z. B. geteilter Renderer, Skripte aus).
    pub fn with_env(host: Arc<dyn SessionHost>, env: ToolEnv) -> FirepixelMcp {
        FirepixelMcp { host, env }
    }

    /// Werkzeugliste im rmcp-Format.
    pub fn tool_list() -> Vec<Tool> {
        tools::tool_defs()
            .into_iter()
            .map(|d| {
                let schema = match d.schema {
                    Value::Object(m) => m,
                    _ => serde_json::Map::new(),
                };
                let ann = ToolAnnotations::new()
                    .read_only(d.read_only)
                    .destructive(d.destructive)
                    .idempotent(d.read_only)
                    .open_world(false);
                Tool::new(d.name, d.description, Arc::new(schema))
                    .with_title(d.title)
                    .with_annotations(ann)
            })
            .collect()
    }
}

fn to_result(out: tools::ToolOutput) -> CallToolResult {
    let content: Vec<ContentBlock> = out
        .blocks
        .into_iter()
        .map(|b| match b {
            Block::Text(t) => ContentBlock::text(t),
            Block::Image(png) => ContentBlock::image(crate::image::base64(&png), "image/png"),
        })
        .collect();
    let mut r = if out.is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    };
    if let Some(v) = out.structured.filter(|v| v.is_object()) {
        r.structured_content = Some(v);
    }
    r
}

impl ServerHandler for FirepixelMcp {
    fn get_info(&self) -> ServerConfig {
        let mut imp = Implementation::new("firepixel", env!("CARGO_PKG_VERSION"));
        imp.title = Some(format!("{} ({})", fp_i18n::DISPLAY_NAME, self.host.mode()));
        let text = fp_i18n::with_language(&self.env.language, || {
            instructions(self.host.mode() == "bridge")
        });
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(imp)
        .with_instructions(text)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(fp_i18n::with_language(
            &self.env.language,
            FirepixelMcp::tool_list,
        )))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        fp_i18n::with_language(&self.env.language, FirepixelMcp::tool_list)
            .into_iter()
            .find(|t| t.name == name)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let name = request.name.to_string();
        if !tools::tool_defs().iter().any(|d| d.name == name) {
            return Err(ErrorData::invalid_params(
                fp_i18n::with_language(&self.env.language, || {
                    fp_i18n::tr!("err-mcp-unknown-tool", name = name.as_str())
                }),
                None,
            ));
        }
        let args = request.arguments.map(Value::Object).unwrap_or(Value::Null);
        if let Some(req) = tools::ui_request_for(&name, &args) {
            let out = match crate::host::ui_request(
                self.host.as_ref(),
                req,
                tools::ui_timeout(&name, &args),
            )
            .await
            {
                Ok(reply) => tools::ui_output(&name, &args, reply),
                Err(e) => tools::error_output(fp_i18n::with_language(&self.env.language, || {
                    e.to_string()
                })),
            };
            return Ok(to_result(out).into());
        }
        let env = self.env.clone();
        let bridge = self.host.mode() == "bridge";
        let out = with_session(self.host.as_ref(), move |s| {
            if bridge {
                // Brücke: Standardnamen neuer Dokumente/Ebenen in der Sprache der Oberfläche (der
                // Auftrag läuft im UI-Thread), Antworten weiter in der Sprache des Agenten (CT-3)
                fp_i18n::with_content_language(&fp_i18n::current_language(), || {
                    tools::call(s, &env, &name, &args)
                })
            } else {
                tools::call(s, &env, &name, &args)
            }
        })
        .await
        .map_err(|e| {
            ErrorData::internal_error(
                fp_i18n::with_language(&self.env.language, || e.to_string()),
                None,
            )
        })?;
        Ok(to_result(out).into())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let list = fp_i18n::with_language(&self.env.language, resources::resources)
            .into_iter()
            .map(|r| {
                Resource::new(r.uri, r.name)
                    .with_title(r.title)
                    .with_description(r.description)
                    .with_mime_type("text/markdown")
            })
            .collect();
        Ok(ListResourcesResult::with_all_items(list))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let uri = request.uri.clone();
        let lang = self.env.language.clone();
        let text = with_session(self.host.as_ref(), move |s| {
            fp_i18n::with_language(&lang, || resources::read(&uri, s.registry()))
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        match text {
            Some(t) => Ok(ReadResourceResult::new(vec![
                ResourceContents::text(t, request.uri).with_mime_type("text/markdown"),
            ])
            .into()),
            None => Err(ErrorData::invalid_params(
                fp_i18n::with_language(&self.env.language, || {
                    fp_i18n::tr!("err-mcp-unknown-resource", uri = request.uri.to_string())
                }),
                None,
            )),
        }
    }
}
